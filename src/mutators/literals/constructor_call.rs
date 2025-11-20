use rand::Rng;
use swc_common::DUMMY_SP;
use swc_ecma_visit::{VisitWith, swc_ecma_ast::*};
use swc_ecma_visit::{VisitMut, VisitMutWith};

use crate::mutators::AstMutator;
use crate::mutators::scope::{CountNumericLiterals, IdentCollector};

/// ConstructorCall mutator
/// Picks random idents and wraps them in a constructor call
/// eg. let x = 5; => let x = new Array(5);
pub struct ConstructorCall;
struct ConstructorCallVisitor {
    idx_to_mutate: usize,
    crt_idx: usize,
    replaced: bool,
}

impl VisitMut for ConstructorCallVisitor {
    fn visit_mut_expr(&mut self, node: &mut Expr) {
        if self.replaced {
            node.visit_mut_children_with(self);
            return;
        }

        if let Expr::Ident(ident) = node {
            if self.crt_idx == self.idx_to_mutate {
                let new_expr = Expr::New(NewExpr {
                    span: DUMMY_SP,
                    ctxt: Default::default(),
                    callee: Box::new(Expr::Ident(Ident {
                        span: DUMMY_SP,
                        sym: "Array".into(),
                        ctxt: Default::default(),
                        optional: false,
                    })),
                    args: Some(vec![ExprOrSpread {
                        spread: None,
                        expr: Box::new(Expr::Ident(ident.clone())),
                    }]),
                    type_args: None,
                });
                *node = new_expr;
                self.replaced = true;
                return;
            }
            self.crt_idx += 1;
        }

        node.visit_mut_children_with(self);
    }
}

impl AstMutator for ConstructorCall {
    fn mutate(&self, mut ast: Script) -> anyhow::Result<Script> {
        // TODO: decide if we want to wrap random Ident or Lit or Expr(maybe)
        // for now just mutate Lit
        let mut rng = rand::rng();
        // if rng.random_bool(1.0) {
        //     let mut collector = CountNumericLiterals { count : 0 };
        //     ast.visit_with(&mut collector);
        //     let lit_count = collector.count;

        // }
        let mut collector = IdentCollector::default();
        ast.visit_with(&mut collector);
        let ident_cnt = collector.idents.len();
        if ident_cnt == 0 {
            return Ok(ast);
        }

        
        let idx_to_mutate = rng.random_range(0..ident_cnt);
        let mut visitor = ConstructorCallVisitor {
            idx_to_mutate,
            crt_idx: 0,
            replaced: false,
        };
        ast.visit_mut_with(&mut visitor);
        Ok(ast)
    }
}
