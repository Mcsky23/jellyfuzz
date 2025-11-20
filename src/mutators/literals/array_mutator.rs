use anyhow::Result;
use rand::Rng;
use rand::seq::IndexedRandom;
use swc_atoms::Atom;
use swc_ecma_visit::{Visit, VisitMut, VisitMutWith};
use swc_ecma_visit::{VisitWith, swc_ecma_ast::*};

use crate::mutators::AstMutator;
use crate::utils::rand_utils::{random_weighted_choice, small_delta};
use crate::mutators::scope::*;

/// ArrayMutator
/// Mutates defined arrays by changing their legnths and/or swapping elements with
/// other variables from the current context.
pub struct ArrayMutator;
pub struct ArrayMutatorVisitor {
    rng: rand::rngs::ThreadRng,
    idx_to_mutate: usize,
    crt_idx: usize,
    scope_state: ScopeState,
}
pub struct CountArrayLiterals {
    pub count: usize,
}

impl Visit for CountArrayLiterals {
    fn visit_array_lit(&mut self, node: &ArrayLit) {
        self.count += 1;
        node.visit_children_with(self);
    }
}

impl ScopedAstVisitor for ArrayMutatorVisitor {
    fn scope_state(&mut self) -> &mut ScopeState {
        &mut self.scope_state
    }
}

impl VisitMut for ArrayMutatorVisitor {
    scoped_visit_mut_methods!();

    fn visit_mut_array_lit(&mut self, node: &mut ArrayLit) {
        node.visit_mut_children_with(self);

        if self.crt_idx != self.idx_to_mutate {
            self.crt_idx += 1;
            return;
        }
        self.crt_idx += 1;

        let original_len = node.elems.len();
        let mut rng = rand::rng();
        let new_len = if rng.random_bool(0.5) {
            // increase length
            original_len + rng.random_range(1..=5)
        } else {
            // decrease length
            if original_len == 0 {
                0
            } else {
                rng.random_range(0..original_len)
            }
        };

        if new_len > original_len {
            // add undefined elements
            // decide what type of elements to add
            let choice = random_weighted_choice(
                &mut self.rng,
                &[
                    ("smi", 30),
                    ("float", 20),
                    ("bigint", 20),
                    ("nan", 5),
                    ("null", 5),
                    ("undefined", 5),
                    ("boolean", 5),
                    ("objects", 10),
                    ("context_obj", 10)
                ],
            );
            match choice {
                "smi" => {
                    for _ in original_len..new_len {
                        let val = rng.random_range(-100i32..=100i32);
                        let lit = Lit::Num(Number {
                            span: Default::default(),
                            value: val as f64,
                            raw: Some(Atom::from(val.to_string().as_str())),
                        });
                        node.elems.push(Some(ExprOrSpread {
                            spread: None,
                            expr: Box::new(Expr::Lit(lit)),
                        }));
                    }
                }
                "float" => {
                    for _ in original_len..new_len {
                        let val = rng.random_range(-100.0f64..=100.0f64);
                        let lit = Lit::Num(Number {
                            span: Default::default(),
                            value: val,
                            raw: Some(Atom::from(format!("{:.2}", val).as_str())),
                        });
                        node.elems.push(Some(ExprOrSpread {
                            spread: None,
                            expr: Box::new(Expr::Lit(lit)),
                        }));
                    }
                }
                "nan" => {
                    for _ in original_len..new_len {
                        let lit = Lit::Num(Number {
                            span: Default::default(),
                            value: f64::NAN,
                            raw: Some(Atom::from("NaN")),
                        });
                        node.elems.push(Some(ExprOrSpread {
                            spread: None,
                            expr: Box::new(Expr::Lit(lit)),
                        }));
                    }
                }
                "undefined" => {
                    for _ in original_len..new_len {
                        let lit = Expr::Ident(Ident {
                            span: Default::default(),
                            sym: Atom::from("undefined"),
                            optional: false,
                            ctxt: Default::default(),
                        });
                        node.elems.push(Some(ExprOrSpread {
                            spread: None,
                            expr: Box::new(lit),
                        }));
                    }
                }
                // pick random stuff from scope and swap in
                "context_obj" => {
                    let obj_list = self.scope_state.scopes.collect_idents_and_functions();
                    if obj_list.len() == 0 {
                        return;
                    }
                    for _ in original_len..new_len {
                        let obj = obj_list.choose(&mut self.rng)
                            .expect("ArrayMutator: Failed to pick random context obj");
                        
                        node.elems.push(Some(ExprOrSpread { spread: None, expr: Box::new(Expr::Ident(obj.clone())) }));
                    }

                }
                _ => {
                    for _ in original_len..new_len {
                        let lit = Expr::Ident(Ident {
                            span: Default::default(),
                            sym: Atom::from("undefined"),
                            optional: false,
                            ctxt: Default::default(),
                        });
                        node.elems.push(Some(ExprOrSpread {
                            spread: None,
                            expr: Box::new(lit),
                        }));
                    }
                }
            }
        } else if new_len < original_len {
            node.elems.truncate(new_len);
        }
    }
}

impl AstMutator for ArrayMutator {
    fn mutate(&self, mut ast: Script) -> Result<Script> {
        let mut counter = CountArrayLiterals { count: 0 };
        ast.visit_with(&mut counter);
        if counter.count == 0 {
            // No array literals to mutate
            return Ok(ast);
        }

        // randomly choose a literal index to mutate
        let mut rng = rand::rng();
        let idx_to_mutate = rng.random_range(0..counter.count);
        let mut visitor = ArrayMutatorVisitor {
            rng,
            idx_to_mutate,
            crt_idx: 0,
            scope_state: ScopeState::new(),
        };
        ast.visit_mut_with(&mut visitor);
        Ok(ast)
    }
}
