use rand::prelude::IndexedRandom;
use rand::Rng;
use swc_atoms::Atom;
use swc_common::DUMMY_SP;
use swc_ecma_visit::{Visit, VisitWith, swc_ecma_ast::*};
use swc_ecma_visit::{VisitMut, VisitMutWith};

use crate::mutators::AstMutator;
use crate::mutators::js_objects::js_objects::{get_global_object, get_random_global_object, JsMethodKind};
use crate::mutators::js_objects::js_types::JsObjectType;
use crate::mutators::scope::IdentCollector;

/// ConstructorCall mutator
/// Picks random idents and wraps them in a constructor call
/// eg. let x = 5; => let x = new Array(5);
pub struct ConstructorCall;
struct ConstructorCallVisitor {
    rng: rand::rngs::ThreadRng,
    idx_to_mutate: usize,
    crt_idx: usize,
    replaced: bool,
    mode: MutatorMode,
    scope_values: Vec<Expr>,
    in_for_header: bool,
}

enum MutatorMode {
    WrapIdent,  // wraps idents in a random constructor
    InsertCode, // chooses a random point to insert a new code line 
    ReplaceDecl, // turns let x = ... into let x = new Object(whatever)
}

#[derive(Default)]
struct DeclCounter {
    count: usize,
}

impl Visit for DeclCounter {
    fn visit_var_declarator(&mut self, node: &VarDeclarator) {
        if node.init.is_some() {
            self.count += 1;
        }
        node.visit_children_with(self);
    }
}

impl VisitMut for ConstructorCallVisitor {
    fn visit_mut_expr(&mut self, node: &mut Expr) {
        match self.mode {
            MutatorMode::WrapIdent => {
                if self.in_for_header {
                    node.visit_mut_children_with(self);
                    return;
                }
                if self.replaced {
                    node.visit_mut_children_with(self);
                    return;
                }
                
                if let Expr::Ident(ident) = node {
                    if self.crt_idx == self.idx_to_mutate {
                        let ctor_name = get_random_global_object(&mut self.rng);
                        let new_expr = Expr::New(NewExpr {
                            span: DUMMY_SP,
                            ctxt: Default::default(),
                            callee: Box::new(Expr::Ident(Ident {
                                span: DUMMY_SP,
                                sym: ctor_name.clone().into(),
                                ctxt: Default::default(),
                                optional: false,
                            })),
                            args: Some(vec![ExprOrSpread {
                                spread: None,
                                expr: Box::new(Expr::Ident(ident.clone())),
                            }]),
                            type_args: None,
                        });
                        let wrapped = self.build_chained_constructors(
                            ctor_name,
                            new_expr,
                            vec![Expr::Ident(ident.clone())],
                        );
                        *node = wrapped;
                        self.replaced = true;
                        return;
                    }
                    self.crt_idx += 1;
                }
                
                node.visit_mut_children_with(self);
            }

            MutatorMode::ReplaceDecl => {
                if self.in_for_header {
                    node.visit_mut_children_with(self);
                    return;
                }
                node.visit_mut_children_with(self);
            }

            _ => {
                unreachable!()
            }
        }
    }

    fn visit_mut_var_declarator(&mut self, node: &mut VarDeclarator) {
        match self.mode {
            MutatorMode::ReplaceDecl => {
                if self.in_for_header {
                    node.visit_mut_children_with(self);
                    return;
                }
                if self.replaced {
                    node.visit_mut_children_with(self);
                    return;
                }

                if node.init.is_some() {
                    if self.crt_idx == self.idx_to_mutate {
                        let original_init = node.init.take().unwrap();
                        let init_clone = (*original_init).clone();
                        let ctor_name = get_random_global_object(&mut self.rng);
                        let new_expr = Expr::New(NewExpr {
                            span: DUMMY_SP,
                            ctxt: Default::default(),
                            callee: Box::new(Expr::Ident(Ident {
                                span: DUMMY_SP,
                                sym: ctor_name.clone().into(),
                                ctxt: Default::default(),
                                optional: false,
                            })),
                            args: Some(vec![ExprOrSpread {
                                spread: None,
                                expr: original_init,
                            }]),
                            type_args: None,
                        });
                        let wrapped = self.build_chained_constructors(ctor_name, new_expr, vec![init_clone]);
                        node.init = Some(Box::new(wrapped));
                        self.replaced = true;
                        return;
                    }
                    self.crt_idx += 1;
                }

                node.visit_mut_children_with(self);
            }
            _ => node.visit_mut_children_with(self),
        }
    }

    fn visit_mut_for_stmt(&mut self, node: &mut ForStmt) {
        // Do not mutate the header (init/test/update).
        let saved = self.in_for_header;
        self.in_for_header = true;
        if let Some(init) = &mut node.init {
            init.visit_mut_children_with(self);
        }
        if let Some(test) = &mut node.test {
            test.visit_mut_children_with(self);
        }
        if let Some(update) = &mut node.update {
            update.visit_mut_children_with(self);
        }
        self.in_for_header = saved;
        node.body.visit_mut_with(self);
    }

    // fn visit_mut_for_in_stmt(&mut self, node: &mut ForInStmt) {
    //     let saved = self.in_for_header;
    //     self.in_for_header = true;
    //     node.left.visit_mut_with(self);
    //     node.right.visit_mut_with(self);
    //     self.in_for_header = saved;
    //     node.body.visit_mut_with(self);
    // }

    // fn visit_mut_for_of_stmt(&mut self, node: &mut ForOfStmt) {
    //     let saved = self.in_for_header;
    //     self.in_for_header = true;
    //     node.left.visit_mut_with(self);
    //     node.right.visit_mut_with(self);
    //     self.in_for_header = saved;
    //     node.body.visit_mut_with(self);
    // }
}

impl ConstructorCallVisitor {
    fn wrap_with_method_calls(
        &mut self,
        ctor_expr: Expr,
        object_sym: &str,
        mut value_pool: Vec<Expr>,
    ) -> Expr {
        value_pool.extend_from_slice(&self.scope_values);
        let Some(obj) = get_global_object(object_sym) else {
            return ctor_expr;
        };

        let mut instance_methods: Vec<_> = obj
            .methods()
            .iter()
            .filter(|m| matches!(m.kind(), JsMethodKind::Instance))
            .cloned()
            .collect();

        if instance_methods.is_empty() {
            return ctor_expr;
        }

        let call_count = std::cmp::min(self.rng.random_range(0..=5), instance_methods.len());

        // build IIFE that constructs the object, calls a few methods, then returns it
        let tmp_ident = Ident {
            span: DUMMY_SP,
            sym: format!("__tmp{}", self.rng.random_range(0..10_000)).into(),
            ctxt: Default::default(),
            optional: false,
        };

        let decl = VarDeclarator {
            span: DUMMY_SP,
            name: Pat::Ident(BindingIdent {
                id: tmp_ident.clone(),
                type_ann: None,
            }),
            init: Some(Box::new(ctor_expr)),
            definite: false,
        };

        let mut stmts: Vec<Stmt> = vec![Stmt::Decl(Decl::Var(Box::new(VarDecl {
            span: DUMMY_SP,
            ctxt: Default::default(),
            kind: VarDeclKind::Const,
            decls: vec![decl],
            declare: false,
        })))];

        value_pool.push(Expr::Ident(tmp_ident.clone()));

        for _ in 0..call_count {
            if let Some(method) = instance_methods.choose(&mut self.rng).cloned() {
                if let Some(sig) = method.signatures().choose(&mut self.rng) {
                    let args = self.build_args(sig.types(), &mut value_pool);
                    let member_ident = IdentName::new(Atom::from(method.sym()), DUMMY_SP);
                    let call_expr = Expr::Call(CallExpr {
                        span: DUMMY_SP,
                        ctxt: Default::default(),
                        callee: Callee::Expr(Box::new(Expr::Member(MemberExpr {
                            span: DUMMY_SP,
                            obj: Box::new(Expr::Ident(tmp_ident.clone())),
                            prop: MemberProp::Ident(member_ident),
                        }))),
                        args,
                        type_args: None,
                    });
                    stmts.push(Stmt::Expr(ExprStmt {
                        span: DUMMY_SP,
                        expr: Box::new(call_expr),
                    }));
                }
            }
        }

        stmts.push(Stmt::Return(ReturnStmt {
            span: DUMMY_SP,
            arg: Some(Box::new(Expr::Ident(tmp_ident))),
        }));

        Expr::Call(CallExpr {
            span: DUMMY_SP,
            ctxt: Default::default(),
            callee: Callee::Expr(Box::new(Expr::Paren(ParenExpr {
                span: DUMMY_SP,
                expr: Box::new(Expr::Arrow(ArrowExpr {
                    span: DUMMY_SP,
                    ctxt: Default::default(),
                    params: Vec::new(),
                    body: Box::new(BlockStmtOrExpr::BlockStmt(BlockStmt {
                        span: DUMMY_SP,
                        ctxt: Default::default(),
                        stmts,
                    })),
                    is_async: false,
                    is_generator: false,
                    type_params: None,
                    return_type: None,
                })),
            }))),
            args: Vec::new(),
            type_args: None,
        })
    }

    fn build_chained_constructors(
        &mut self,
        ctor_name: String,
        ctor_expr: Expr,
        value_pool: Vec<Expr>,
    ) -> Expr {
        let depth = self.rng.random_range(0..=4);
        let mut current_expr = self.wrap_with_method_calls(ctor_expr, &ctor_name, value_pool.clone());

        for _ in 1..depth {
            let next_ctor_name = get_random_global_object(&mut self.rng);
            let new_ctor = Expr::New(NewExpr {
                span: DUMMY_SP,
                ctxt: Default::default(),
                callee: Box::new(Expr::Ident(Ident {
                    span: DUMMY_SP,
                    sym: next_ctor_name.clone().into(),
                    ctxt: Default::default(),
                    optional: false,
                })),
                args: Some(vec![ExprOrSpread {
                    spread: None,
                    expr: Box::new(current_expr),
                }]),
                type_args: None,
            });
            current_expr =
                self.wrap_with_method_calls(new_ctor, &next_ctor_name, value_pool.clone());
        }

        current_expr
    }

    fn build_args(
        &mut self,
        types: &[JsObjectType],
        value_pool: &mut Vec<Expr>,
    ) -> Vec<ExprOrSpread> {
        types
            .iter()
            .map(|ty| {
                let expr = self.build_arg_expr(*ty, value_pool);
                value_pool.push(expr.clone());
                ExprOrSpread {
                    spread: None,
                    expr: Box::new(expr),
                }
            })
            .collect()
    }

    fn build_arg_expr(&mut self, ty: JsObjectType, value_pool: &[Expr]) -> Expr {
        if !value_pool.is_empty() && self.rng.random_bool(0.35) {
            if let Some(existing) = value_pool.choose(&mut self.rng) {
                return existing.clone();
            }
        }

        match ty {
            JsObjectType::Number => {
                // Bias toward interesting edge numbers.
                let specials = [0.0f64, -0.0, -1.0, 1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 0xffff_ffffu32 as f64];
                let v = if self.rng.random_bool(0.35) {
                    *specials.choose(&mut self.rng).unwrap_or(&0.0)
                } else {
                    self.rng.random_range(-16i32..=16i32) as f64
                };
                Expr::Lit(Lit::Num(Number {
                    span: DUMMY_SP,
                    value: v,
                    raw: None,
                }))
            }
            JsObjectType::JsString => {
                let choices = ["foo", "bar", "baz", "qux", "こんにちは"];
                let s = choices.choose(&mut self.rng).copied().unwrap_or("s");
                Expr::Lit(Lit::Str(Str {
                    span: DUMMY_SP,
                    value: Atom::from(s).into(),
                    raw: None,
                }))
            }
            JsObjectType::Object => Expr::Object(ObjectLit {
                span: DUMMY_SP,
                props: Vec::new(),
            }),
            JsObjectType::Array => Expr::Array(ArrayLit {
                span: DUMMY_SP,
                elems: Vec::new(),
            }),
            JsObjectType::Function => {
                // Prefer reusing a value from scope/value pool when asked for a function.
                if !value_pool.is_empty() && self.rng.random_bool(0.5) {
                    if let Some(existing) = value_pool.choose(&mut self.rng) {
                        return existing.clone();
                    }
                }

                Expr::Fn(FnExpr {
                    ident: None,
                    function: Box::new(Function {
                        params: Vec::new(),
                        decorators: Vec::new(),
                        span: DUMMY_SP,
                        ctxt: Default::default(),
                        body: Some(BlockStmt {
                            span: DUMMY_SP,
                            ctxt: Default::default(),
                            stmts: Vec::new(),
                        }),
                        is_generator: false,
                        is_async: false,
                        type_params: None,
                        return_type: None,
                    }),
                })
            }
            JsObjectType::Any => {
                let choice = self.rng.random_range(0..5);
                let concrete = match choice {
                    0 => JsObjectType::Number,
                    1 => JsObjectType::JsString,
                    2 => JsObjectType::Object,
                    3 => JsObjectType::Function,
                    _ => JsObjectType::Array,
                };
                self.build_arg_expr(concrete, value_pool)
            }
        }
    }
}

impl AstMutator for ConstructorCall {
    fn mutate(&self, mut ast: Script) -> anyhow::Result<Script> {
        let mut rng = rand::rng();
        let mut ident_collector = IdentCollector::default();
        ast.visit_with(&mut ident_collector);
        let scope_values: Vec<Expr> = ident_collector
            .idents
            .iter()
            .map(|id| Expr::Ident(id.clone()))
            .collect();
        match rng.random_range(0..3) {
            // WrapIdent
            0 => {
                let mut collector = IdentCollector::default();
                ast.visit_with(&mut collector);
                let ident_cnt = collector.idents.len();
                if ident_cnt == 0 {
                    return Ok(ast);
                }
                let idx_to_mutate = rng.random_range(0..ident_cnt);
                let mut visitor = ConstructorCallVisitor {
                    rng,
                    idx_to_mutate,
                    crt_idx: 0,
                    replaced: false,
                    mode: MutatorMode::WrapIdent,
                    scope_values: scope_values.clone(),
                    in_for_header: false,
                };
                ast.visit_mut_with(&mut visitor);
                Ok(ast)
            }
            // // InsertCode
            // 1 => {
            //     todo!();
            // }
            // ReplaceDecl
            1 | 2 => {
                let mut collector = DeclCounter::default();
                ast.visit_with(&mut collector);
                if collector.count == 0 {
                    return Ok(ast);
                }

                let idx_to_mutate = rng.random_range(0..collector.count);
                let mut visitor = ConstructorCallVisitor {
                    rng,
                    idx_to_mutate,
                    crt_idx: 0,
                    replaced: false,
                    mode: MutatorMode::ReplaceDecl,
                    scope_values: scope_values.clone(),
                    in_for_header: false,
                };
                ast.visit_mut_with(&mut visitor);
                Ok(ast)
            }
            _ => {
                unreachable!();
            }
        }
        // if rng.random_bool(1.0) {
        //     let mut collector = CountNumericLiterals { count : 0 };
        //     ast.visit_with(&mut collector);
        //     let lit_count = collector.count;
        
        // }
        
    }
}
