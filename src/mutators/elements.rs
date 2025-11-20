use anyhow::Result;
use rand::Rng;
use rand::seq::IndexedRandom;
use swc_atoms::Atom;
use swc_common::{DUMMY_SP, SyntaxContext};
use swc_ecma_visit::{Visit, VisitMut, VisitMutWith};
use swc_ecma_visit::{VisitWith, swc_ecma_ast::*};

use crate::mutators::AstMutator;
use crate::mutators::js_objects::js_types::{JsObjectType, STATIC_PROPERTIES, StaticPropertySig, get_property_list};
use crate::mutators::scope::{
    IdentCollector, ScopeState, ScopedAstVisitor, for_stmt_visitor, scoped_for_stmt_visitor, scoped_visit_mut_methods
};

pub struct ElementAccessorMutator;
pub struct MethodCallMutator;

fn number_expr(value: f64) -> Expr {
    let raw_string = if value.fract() == 0.0 {
        format!("{}", value as i64)
    } else {
        value.to_string()
    };
    Expr::Lit(Lit::Num(Number {
        span: DUMMY_SP,
        value,
        raw: Some(Atom::from(raw_string)),
    }))
}

struct ElementAccessorVisitor {
    rng: rand::rngs::ThreadRng,
    target_idx: usize,
    current_idx: usize,
    replaced: bool,
    scope_state: ScopeState,
}

impl ScopedAstVisitor for ElementAccessorVisitor {
    fn scope_state(&mut self) -> &mut ScopeState {
        &mut self.scope_state
    }
}

impl ElementAccessorVisitor {
    fn new(rng: rand::rngs::ThreadRng, target_idx: usize) -> Self {
        Self {
            rng,
            target_idx,
            current_idx: 0,
            replaced: false,
            scope_state: ScopeState::new(),
        }
    }

    fn pick_scope_ident(&mut self) -> Option<Ident> {
        let candidates = self.scope_stack().collect_idents();
        candidates.choose(&mut self.rng).cloned()
    }

    fn random_index_literal(&mut self) -> Expr {
        let value = self.rng.random_range(0..=5) as f64;
        number_expr(value)
    }

    fn build_accessor_expr(&mut self, base: Ident) -> Expr {
        if self.rng.random_bool(0.5) {
            self.build_index_access(base)
        } else {
            self.build_property_access(base)
        }
    }

    fn build_index_access(&mut self, base: Ident) -> Expr {
        let use_numeric = self.rng.random_bool(0.5);
        let index_expr = if !use_numeric {
            if let Some(ident) = self.pick_scope_ident() {
                Expr::Ident(ident)
            } else {
                self.random_index_literal()
            }
        } else {
            self.random_index_literal()
        };

        Expr::Member(MemberExpr {
            span: base.span,
            obj: Box::new(Expr::Ident(base)),
            prop: MemberProp::Computed(ComputedPropName {
                span: DUMMY_SP,
                expr: Box::new(index_expr),
            }),
        })
    }

    fn build_property_access(&mut self, base: Ident) -> Expr {
        if self.rng.random_bool(0.4) {
            if let Some(ident) = self.pick_scope_ident() {
                return Expr::Member(MemberExpr {
                    span: base.span,
                    obj: Box::new(Expr::Ident(base)),
                    prop: MemberProp::Computed(ComputedPropName {
                        span: DUMMY_SP,
                        expr: Box::new(Expr::Ident(ident)),
                    }),
                });
            }
        }

        let property = get_property_list()
            .choose(&mut self.rng)
            .copied()
            .unwrap_or("__proto__");

        let member_ident = IdentName::new(Atom::from(property), DUMMY_SP);

        Expr::Member(MemberExpr {
            span: base.span,
            obj: Box::new(Expr::Ident(base)),
            prop: MemberProp::Ident(member_ident),
        })
    }
}

impl VisitMut for ElementAccessorVisitor {
    scoped_visit_mut_methods!();
    scoped_for_stmt_visitor!(mut);

    fn visit_mut_expr(&mut self, node: &mut Expr) {
        if self.in_for_stmt().is_some() {
            node.visit_mut_children_with(self);
            return;
        }

        if self.replaced {
            node.visit_mut_children_with(self);
            return;
        }

        if let Expr::Ident(ident) = node {
            let idx = self.current_idx;
            self.current_idx += 1;

            if idx == self.target_idx {
                let replacement = self.build_accessor_expr(ident.clone());
                *node = replacement;
                self.replaced = true;
                return;
            }
        } else {
            node.visit_mut_children_with(self);
            return;
        }

        node.visit_mut_children_with(self);
    }
}

impl AstMutator for ElementAccessorMutator {
    fn mutate(&self, mut ast: Script) -> Result<Script> {
        let mut collector = IdentCollector::default();
        ast.visit_with(&mut collector);
        let mut rng = rand::rng();
        let ident_count = collector.idents.len();
        if ident_count > 0 {
            let target_idx = rng.random_range(0..ident_count);
            let mut visitor = ElementAccessorVisitor::new(rng, target_idx);
            ast.visit_mut_with(&mut visitor);
        }

        Ok(ast)
    }
}

fn string_expr_from_atom(value: &str) -> Expr {
    Expr::Lit(Lit::Str(Str {
        span: DUMMY_SP,
        value: Atom::from(value).into(),
        raw: None,
    }))
}

fn empty_function_expr() -> Expr {
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

struct MethodCallVisitor {
    rng: rand::rngs::ThreadRng,
    target_idx: usize,
    current_idx: usize,
    replaced: bool,
    in_for_stmt: Option<&'static str>,
}

impl MethodCallVisitor {
    fn new(rng: rand::rngs::ThreadRng, target_idx: usize) -> Self {
        Self {
            rng,
            target_idx,
            current_idx: 0,
            replaced: false,
            in_for_stmt: None,
        }
    }

    fn build_synthetic_base(&mut self) -> Expr {
        match self.rng.random_range(0..3) {
            // Plain object {}.
            0 => Expr::Object(ObjectLit {
                span: DUMMY_SP,
                props: Vec::new(),
            }),
            // Empty array [].
            1 => Expr::Array(ArrayLit {
                span: DUMMY_SP,
                elems: Vec::new(),
            }),
            // Empty function () => {}.
            _ => empty_function_expr(),
        }
    }

    fn build_arg_expr(&mut self, ty: JsObjectType) -> Expr {
        match ty {
            JsObjectType::Number => {
                let v = self.rng.random_range(-100i32..=100i32) as f64;
                number_expr(v)
            }
            JsObjectType::JsString => {
                let choices = ["foo", "bar", "baz", "qux"];
                let s = choices
                    .choose(&mut self.rng)
                    .copied()
                    .unwrap_or("s");
                string_expr_from_atom(s)
            }
            JsObjectType::Object => Expr::Object(ObjectLit {
                span: DUMMY_SP,
                props: Vec::new(),
            }),
            JsObjectType::Function => empty_function_expr(),
            JsObjectType::Any => {
                // Pick a concrete type for Any.
                let concrete = match self.rng.random_range(0..4) {
                    0 => JsObjectType::Number,
                    1 => JsObjectType::JsString,
                    2 => JsObjectType::Object,
                    _ => JsObjectType::Function,
                };
                self.build_arg_expr(concrete)
            }
        }
    }

    fn build_method_call(&mut self, base: Expr) -> Expr {
        let methods: Vec<&StaticPropertySig> = STATIC_PROPERTIES
            .iter()
            .filter(|sig| sig.is_method)
            .collect();
        if methods.is_empty() {
            return base;
        }
        let sig = methods
            .choose(&mut self.rng)
            .copied()
            .unwrap_or(&STATIC_PROPERTIES[0]);

        // With some probability, call the method on a synthetic base
        // (e.g. {}, [], function() {}), otherwise use the identifier
        // from the current scope.
        let base_expr = if self.rng.random_bool(0.5) {
            base
        } else {
            self.build_synthetic_base()
        };

        let member_ident = IdentName::new(Atom::from(sig.name), DUMMY_SP);
        let callee_expr = Expr::Member(MemberExpr {
            span: DUMMY_SP,
            obj: Box::new(base_expr),
            prop: MemberProp::Ident(member_ident),
        });

        let args: Vec<ExprOrSpread> = sig
            .args
            .iter()
            .map(|ty| ExprOrSpread {
                spread: None,
                expr: Box::new(self.build_arg_expr(*ty)),
            })
            .collect();

        Expr::Call(CallExpr {
            span: DUMMY_SP,
            ctxt: Default::default(),
            callee: Callee::Expr(Box::new(callee_expr)),
            args,
            type_args: None,
        })
    }
}

impl VisitMut for MethodCallVisitor {
    for_stmt_visitor!(mut);

    fn visit_mut_expr(&mut self, node: &mut Expr) {
        // Avoid modifying expressions that are part of for-loop headers
        // (init / test / update), as those are a common source of timeouts
        // when aggressively mutated.
        if self.in_for_stmt.is_some() {
            node.visit_mut_children_with(self);
            return;
        }

        if self.replaced {
            node.visit_mut_children_with(self);
            return;
        }

        if let Expr::Ident(ident) = node {
            let idx = self.current_idx;
            self.current_idx += 1;
            if idx == self.target_idx {
                let base = Expr::Ident(ident.clone());
                let call_expr = self.build_method_call(base);

                *node = call_expr;
                self.replaced = true;
                return;
            }
        }

        node.visit_mut_children_with(self);
    }
}

impl AstMutator for MethodCallMutator {
    fn mutate(&self, mut ast: Script) -> Result<Script> {
        let mut collector = IdentCollector::default();
        ast.visit_with(&mut collector);
        let mut rng = rand::rng();
        let ident_count = collector.idents.len();
        if ident_count > 0 {
            let target_idx = rng.random_range(0..ident_count);
            let mut visitor = MethodCallVisitor::new(rng, target_idx);
            ast.visit_mut_with(&mut visitor);
        }
        Ok(ast)
    }
}


/// Removes an element accessor from an expression like `obj[index]` or `obj.prop`,
/// converting it back to a simple identifier `obj`.
pub struct RemovePropMutator;
impl AstMutator for RemovePropMutator {
    fn mutate(&self, mut ast: Script) -> Result<Script> {
        struct RemovePropVisitor {
            counter_mode: bool,
            counter: usize,
            idx_to_remove: usize,
        }

        impl VisitMut for RemovePropVisitor {
            fn visit_mut_expr(&mut self, node: &mut Expr) {
                if let Expr::Member(MemberExpr { obj, .. }) = node {
                    if let Expr::Ident(_) = **obj {
                        self.counter += 1;
                        if !self.counter_mode && self.counter == self.idx_to_remove {
                            // Replace the member expression with its base identifier.
                            *node = *obj.clone();
                            return;
                        }
                    }
                }
                node.visit_mut_children_with(self);
            }
        }

        let mut collector = RemovePropVisitor {
            counter_mode: true,
            counter: 0,
            idx_to_remove: 0,
        };
        ast.visit_mut_with(&mut collector);

        if collector.counter == 0 {
            return Ok(ast);
        }

        let mut remover = RemovePropVisitor {
            counter_mode: false,
            counter: 0,
            idx_to_remove: rand::rng().random_range(0..collector.counter),
        };
        ast.visit_mut_with(&mut remover);

        Ok(ast)
    }
}
