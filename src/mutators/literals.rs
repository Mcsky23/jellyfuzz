use anyhow::Result;
use rand::seq::IndexedRandom;
use rand::{thread_rng, Rng};
use swc_atoms::Atom;
use swc_ecma_visit::{VisitWith, swc_ecma_ast::*};
use swc_ecma_visit::{VisitMut, VisitMutWith, Visit};
use crate::mutators::AstMutator;
use crate::utils::rand_utils::{boolean_with_probability, gaussian_sample, random_weighted_choice, small_delta};

/// NumericTweaker — improved version
pub struct NumericTweaker;

struct NumericTweakerVisitor {
    rng: rand::rngs::ThreadRng,
    idx_to_mutate: usize,
    crt_idx: usize,
}

impl NumericTweakerVisitor {
    fn new(lit_count: usize) -> Self {
        let mut rng = thread_rng();
        let idx_to_mutate = rng.gen_range(0..lit_count);
        println!("NumericTweaker: chosen literal index to mutate: {}", idx_to_mutate);
        Self {
            rng,
            idx_to_mutate,
            crt_idx: 0,
        }
    }


    /// Choose a random power-of-two value (2^n) within a safe exponent range.
    fn random_pow2(&mut self) -> f64 {
        let exp = self.rng.gen_range(0i32..=60i32);
        2f64.powi(exp)
    }

    fn format_number_raw(&self, value: f64) -> String {
        if value.is_nan() {
            return "NaN".to_string();
        }
        if value.is_infinite() {
            return if value.is_sign_positive() {
                "Infinity".into()
            } else {
                "-Infinity".into()
            };
        }
        if value == 0.0 && value.is_sign_negative() {
            return "-0".to_string();
        }

        // If it's an integer-valued float, emit as integer
        if value.fract() == 0.0 {
            if value.abs() <= (i64::MAX as f64) {
                return format!("{}", value as i64);
            } else {
                return format!("{:.0e}", value);
            }
        }

        // Otherwise print with fixed precision and trim trailing zeros.
        let mut s = format!("{:.12}", value);
        while s.contains('.') && (s.ends_with('0') || s.ends_with('.')) {
            s.pop();
            if s.ends_with('.') { s.pop(); break; }
        }
        s
    }
}

struct CountNumericLiterals {
    pub count: usize,
}
impl Visit for CountNumericLiterals {
    fn visit_lit(&mut self, node: &Lit) {
        if let Lit::Num(_) = node {
            self.count += 1;
        }
        node.visit_children_with(self);
    }
}

impl VisitMut for NumericTweakerVisitor {
    fn visit_mut_lit(&mut self, node: &mut Lit) {
        node.visit_mut_children_with(self);

        if self.crt_idx != self.idx_to_mutate {
            self.crt_idx += 1;
            return;
        }
        self.crt_idx += 1;

        if let Lit::Num(num_lit) = node {
            let original = num_lit.value;
            let mut new_value = original;

            // Weighted choice of mutation mode
            let choice = random_weighted_choice(&[
                ("small_delta", 15),
                ("add_one", 15),
                ("sub_one", 15),
                ("flip_sign", 10),
                ("to_nan", 6),
                ("to_infinity", 4),
                ("to_neg_infinity", 4),
                ("to_neg_zero", 6),
                ("to_extreme_large", 4),
                ("to_extreme_small", 4),
                ("pow2", 4),
                ("random_fraction", 5),
                ("truncate_int", 4),
                ("scale_mult", 4),
            ]);

            match choice {
                "small_delta" => {
                    new_value += small_delta(&mut self.rng, original);
                }
                "add_one" => {
                    new_value += 1.0;
                }
                "sub_one" => {
                    new_value -= 1.0;
                }
                "flip_sign" => {
                    new_value = -new_value;
                }
                "to_nan" => {
                    new_value = f64::NAN;
                }
                "to_infinity" => {
                    new_value = f64::INFINITY;
                }
                "to_neg_infinity" => {
                    new_value = f64::NEG_INFINITY;
                }
                "to_neg_zero" => {
                    new_value = -0.0;
                }
                "to_extreme_large" => {
                    // sample a large magnitude but not always maximal
                    let mag = [1e100_f64, 1e200_f64, 1e308_f64];
                    new_value = *mag.choose(&mut self.rng).unwrap();
                }
                "to_extreme_small" => {
                    let mag = [1e-100_f64, 1e-200_f64, 1e-308_f64];
                    new_value = *mag.choose(&mut self.rng).unwrap();
                }
                "pow2" => {
                    new_value = self.random_pow2();
                    // sometimes negative pow2
                    if self.rng.gen_bool(0.1) { new_value = -new_value; }
                }
                "random_fraction" => {
                    new_value = self.rng.gen_range(0.0f64..=1.0f64);
                }
                "truncate_int" => {
                    new_value = new_value.trunc();
                }
                "scale_mult" => {
                    let factor = if self.rng.gen_bool(0.5) { 0.5 } else { 2.0 };
                    new_value *= factor;
                }
                _ => {}
            }

            // Ensure we actually changed something
            let changed = if new_value.is_nan() && !original.is_nan() {
                true
            } else {
                new_value != original
            };

            if changed {
                num_lit.value = new_value;
                let raw = self.format_number_raw(new_value);
                num_lit.raw = Some(Atom::from(raw.as_str()));
            }
        }
    }
}

impl AstMutator for NumericTweaker {
    fn mutate(mut ast: Script) -> Result<Script> {
        let mut counter = CountNumericLiterals { count: 0 };
        ast.visit_with(&mut counter);

        // randomly choose a literal index to mutate
        let mut visitor = NumericTweakerVisitor::new(
            counter.count
        );
        ast.visit_mut_with(&mut visitor);

        // TODO: log telemetry about mutations

        Ok(ast)
    }
}