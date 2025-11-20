use anyhow::Result;
use rand::Rng;
use rand::seq::IndexedRandom;
use swc_atoms::Atom;
use swc_ecma_visit::{Visit, VisitMut, VisitMutWith};
use swc_ecma_visit::{VisitWith, swc_ecma_ast::*};

use crate::mutators::AstMutator;
use crate::utils::rand_utils::{random_weighted_choice, small_delta};

/// BoleanFlipper
/// Flips boolean literals (true -> false, false -> true)
pub struct BooleanFlipper;
pub struct BooleanFlipperVisitor {
    idx_to_mutate: usize,
    crt_idx: usize,
}
pub struct CountBooleanLiterals {
    pub count: usize,
}

impl Visit for CountBooleanLiterals {
    fn visit_lit(&mut self, node: &Lit) {
        if let Lit::Bool(_) = node {
            self.count += 1;
        }
        node.visit_children_with(self);
    }
}

impl VisitMut for BooleanFlipperVisitor {
    fn visit_mut_lit(&mut self, node: &mut Lit) {
        node.visit_mut_children_with(self);

        if let Lit::Bool(bool_lit) = node {
            if self.crt_idx != self.idx_to_mutate {
                self.crt_idx += 1;
                return;
            }
            self.crt_idx += 1;

            bool_lit.value = !bool_lit.value;
        }
    }
}

impl AstMutator for BooleanFlipper {
    fn mutate(&self, mut ast: Script) -> Result<Script> {
        let mut counter = CountBooleanLiterals { count: 0 };
        ast.visit_with(&mut counter);
        if counter.count == 0 {
            // No boolean literals to mutate
            return Ok(ast);
        }

        // randomly choose a literal index to mutate
        let mut rng = rand::rng();
        let idx_to_mutate = rng.random_range(0..counter.count);
        let mut visitor = BooleanFlipperVisitor {
            idx_to_mutate,
            crt_idx: 0,
        };
        ast.visit_mut_with(&mut visitor);
        Ok(ast)
    }
}