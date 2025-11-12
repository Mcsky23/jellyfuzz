use anyhow::Result;
use rand::Rng;
use rand::seq::IndexedRandom;
use swc_atoms::Atom;
use swc_ecma_visit::{Visit, VisitMut, VisitMutWith};
use swc_ecma_visit::{VisitWith, swc_ecma_ast::*};

use crate::mutators::AstMutator;
use crate::mutators::scope::for_stmt_visitor;
use crate::utils::rand_utils::{random_weighted_choice, small_delta};

// TODO: have a bias against swapping operator in for loops' init/test/update?
/// OperatorSwap
/// This mutator swaps binary operators with others of the same category.
/// For example, + can be swapped with -, * with /, && with ||, etc.
pub struct OperatorSwap;
pub struct CountOperators {
    pub count: usize,
    in_for_stmt: Option<&'static str>,
}
pub struct OperatorSwapVisitor {
    rng: rand::rngs::ThreadRng,
    idx_to_mutate: usize,
    current_idx: usize,
    in_for_stmt: Option<&'static str>,
}

impl Visit for CountOperators {
    for_stmt_visitor!();
    fn visit_bin_expr(&mut self, n: &BinExpr) {
        if self.in_for_stmt.is_none() {
            self.count += 1;
            n.visit_children_with(self);
        }
    }
}

const OPS_GROUPS: &[&[BinaryOp]] = &[
    &[
        op!(bin, "+"),
        op!(bin, "-"),
        op!("*"),
        op!("/"),
        op!("%"),
        op!("**"),
    ],
    &[op!("&&"), op!("||")],
    &[
        op!("|"),
        op!("&"),
        op!("^"),
        op!("<<"),
        op!(">>"),
        op!(">>>"),
    ],
    &[
        op!("=="),
        op!("!="),
        op!("==="),
        op!("!=="),
        op!("<"),
        op!("<="),
        op!(">"),
        op!(">="),
    ],
    &[op!("in"), op!("instanceof")],
];

impl VisitMut for OperatorSwapVisitor {
    for_stmt_visitor!(mut);

    fn visit_mut_bin_expr(&mut self, n: &mut BinExpr) {
        n.visit_mut_children_with(self);

        if self.in_for_stmt.is_some() {
            return;
        }

        if self.current_idx == self.idx_to_mutate {
            for group in OPS_GROUPS {
                if let Some(pos) = group.iter().position(|&op| op == n.op) {
                    // 15% chance to change the operator cross-group
                    if self.rng.gen_bool(0.15) {
                        // choose a random group
                        let other_groups: Vec<&[BinaryOp]> =
                            OPS_GROUPS.iter().filter(|&g| g != group).cloned().collect();
                        let new_group = other_groups
                            .choose(&mut self.rng)
                            .expect("there should be other groups");
                        let new_op = new_group
                            .choose(&mut self.rng)
                            .expect("there should be at least one operator in the group");
                        n.op = *new_op;
                        break;
                    }

                    // choose a different operator from the same group
                    let mut choices: Vec<(BinaryOp, f64)> = group
                        .iter()
                        .filter(|&&op| op != n.op)
                        .map(|&op| (op, 1.0))
                        .collect();
                    // TODO: slightly favor operators that are closer in the list
                    // for now, just choose uniformly
                    let total = choices.len() as f64;
                    choices
                        .iter_mut()
                        .for_each(|(_, weight)| *weight = 1.0 / total * 100.0);

                    let new_op = random_weighted_choice(&mut self.rng, &choices);
                    n.op = new_op;
                }
            }
        }
        self.current_idx += 1;
    }
}

impl AstMutator for OperatorSwap {
    fn mutate(&self, ast: Script) -> Result<Script> {
        let mut counter = CountOperators { count: 0, in_for_stmt: None };
        ast.visit_with(&mut counter);
        if counter.count == 0 {
            return Ok(ast);
        }
        let mut rng = rand::rng();
        let idx_to_mutate = rng.random_range(0..counter.count);
        let mut visitor = OperatorSwapVisitor {
            rng,
            idx_to_mutate,
            current_idx: 0,
            in_for_stmt: None,
        };
        let mut ast = ast;
        ast.visit_mut_with(&mut visitor);
        Ok(ast)
    }
}
