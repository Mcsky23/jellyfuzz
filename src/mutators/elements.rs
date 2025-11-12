use anyhow::Result;
use rand::Rng;
use rand::seq::IndexedRandom;
use swc_atoms::Atom;
use swc_common::{DUMMY_SP, SyntaxContext};
use swc_ecma_visit::{Visit, VisitMut, VisitMutWith};
use swc_ecma_visit::{VisitWith, swc_ecma_ast::*};

use crate::mutators::AstMutator;
use crate::mutators::scope::{
    ScopeKind, ScopeStack, ScopedAstVisitor, extend_params_from_fn_params, for_stmt_visitor, scoped_visit_mut_methods
};

const STATIC_PROPERTIES: &[&str] = &["__proto__", "__length__", "foo"];

pub struct ElementAccessorMutator;

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

#[derive(Default)]
struct IdentCollector {
    idents: Vec<Ident>,
    in_for_stmt: Option<&'static str>,
}

impl Visit for IdentCollector {
    for_stmt_visitor!();

    fn visit_expr(&mut self, node: &Expr) {
        if let Expr::Ident(ident) = node {
            self.idents.push(ident.clone());
        }
        node.visit_children_with(self);
    }
}

struct ElementAccessorVisitor {
    rng: rand::rngs::ThreadRng,
    target_idx: usize,
    current_idx: usize,
    replaced: bool,
    scopes: ScopeStack,
    pending_function_names: Vec<Option<Ident>>,
    in_for_stmt: Option<&'static str>,
}

impl ScopedAstVisitor for ElementAccessorVisitor {
    fn scope_stack(&mut self) -> &mut ScopeStack {
        &mut self.scopes
    }

    fn pending_function_names(&mut self) -> &mut Vec<Option<Ident>> {
        &mut self.pending_function_names
    }
}

impl ElementAccessorVisitor {
    fn new(rng: rand::rngs::ThreadRng, target_idx: usize) -> Self {
        Self {
            rng,
            target_idx,
            current_idx: 0,
            replaced: false,
            scopes: ScopeStack::new(),
            pending_function_names: Vec::new(),
            in_for_stmt: None,
        }
    }

    fn pick_scope_ident(&mut self) -> Option<Ident> {
        let candidates = self.scopes.collect_idents();
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

        let property = STATIC_PROPERTIES
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
    for_stmt_visitor!(mut);

    fn visit_mut_expr(&mut self, node: &mut Expr) {
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

    fn visit_mut_function(&mut self, node: &mut Function) {
        let fn_name = self.pending_function_names.pop().flatten();

        self.scopes.push_scope(ScopeKind::Function);

        if let Some(name) = fn_name.clone() {
            self.scopes.add_ident_to_current(name);
        }

        extend_params_from_fn_params(&mut self.scopes, &node.params);

        for param in &mut node.params {
            param.visit_mut_with(self);
        }

        if let Some(body) = &mut node.body {
            body.visit_mut_with(self);
        }

        self.scopes.pop_scope();
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
