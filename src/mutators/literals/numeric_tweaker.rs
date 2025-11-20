use anyhow::Result;
use rand::Rng;
use rand::seq::IndexedRandom;
use swc_atoms::Atom;
use swc_ecma_visit::{Visit, VisitMut, VisitMutWith};
use swc_ecma_visit::{VisitWith, swc_ecma_ast::*};

use crate::mutators::AstMutator;
use crate::mutators::scope::CountNumericLiterals;
use crate::utils::rand_utils::{random_weighted_choice, small_delta};

/// NumericTweaker
/// TODO: I am getting a lot of timeouts when modifying for loop counters. Maybe avoid mutating those or
/// detect them and mutate to smaller ranges?
pub struct NumericTweaker;

struct NumericTweakerVisitor {
    rng: rand::rngs::ThreadRng,
    idx_to_mutate: usize,
    crt_idx: usize,
    in_for_stmt: Option<&'static str>, // know if I'm visiting the literals of the init/test/update of a for statement
    in_array_index: bool,              // true while visiting a computed member index, e.g. arr[<here>]
}

impl NumericTweakerVisitor {
    const FOR_TEST_MAX_ABS: f64 = 1_000.0;
    const ARRAY_INDEX_MAX: f64 = 1_024.0;

    fn new(lit_count: usize) -> Self {
        let mut rng = rand::rng();
        let idx_to_mutate = rng.random_range(0..lit_count);
        // println!(
        //     "NumericTweaker: chosen literal index to mutate: {}",
        //     idx_to_mutate
        // );
        Self {
            rng,
            idx_to_mutate,
            crt_idx: 0,
            in_for_stmt: None,
            in_array_index: false,
        }
    }

    fn clamp_for_test_bound(&self, original: f64, value: f64) -> f64 {
        let mut v = if value.is_finite() {
            value
        } else {
            // Fall back to original if we produced NaN/Infinity.
            original
        };

        // Clamp to a reasonable range to avoid very slow loops.
        v = v.clamp(-Self::FOR_TEST_MAX_ABS, Self::FOR_TEST_MAX_ABS);

        // Test bounds are typically non‑negative.
        if v < 0.0 {
            v = 0.0;
        }

        // Avoid turning a clearly bounded loop into a no‑op.
        if v == 0.0 && original > 0.0 {
            v = 1.0;
        }

        v
    }

    /// Choose a random power-of-two value (2^n) within a safe exponent range.
    fn random_pow2(&mut self) -> f64 {
        let exp = self.rng.random_range(0i32..=60i32);
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
            if s.ends_with('.') {
                s.pop();
                break;
            }
        }
        s
    }
}

impl VisitMut for NumericTweakerVisitor {
    fn visit_mut_for_stmt(&mut self, node: &mut ForStmt) {
        let prev_in_for = self.in_for_stmt;
        if let Some(init) = &mut node.init {
            self.in_for_stmt = Some("init");
            init.visit_mut_with(self);
        }
        if let Some(test) = &mut node.test {
            self.in_for_stmt = Some("test");
            test.visit_mut_with(self);
        }
        if let Some(update) = &mut node.update {
            self.in_for_stmt = Some("update");
            update.visit_mut_with(self);
        }
        self.in_for_stmt = None;
        node.body.visit_mut_with(self);
        self.in_for_stmt = prev_in_for;
    }

    fn visit_mut_member_expr(&mut self, node: &mut MemberExpr) {
        // Visit the object part normally.
        node.obj.visit_mut_with(self);

        // When visiting a computed property (`obj[expr]`), mark any numeric
        // literals inside `expr` as array indices so we can bias them towards
        // small values and avoid huge, timeout‑prone indices.
        let prev_in_array_index = self.in_array_index;
        if let MemberProp::Computed(comp) = &mut node.prop {
            self.in_array_index = true;
            comp.visit_mut_with(self);
            self.in_array_index = prev_in_array_index;
        } else {
            node.prop.visit_mut_with(self);
        }
    }

    fn visit_mut_lit(&mut self, node: &mut Lit) {
        node.visit_mut_children_with(self);

        // println!("{:?}", self.in_for_stmt);

        if let Lit::Null(_) = node {
            // TODO: see what's up with this
            // println!("Found undefined literal");
        }
        if let Lit::Num(num_lit) = node {
            if self.crt_idx != self.idx_to_mutate {
                self.crt_idx += 1;
                return;
            }
            self.crt_idx += 1;

            let original = num_lit.value;
            let mut new_value = original;

            // Special handling for for loops
            if let Some(ctx) = self.in_for_stmt {
                match ctx {
                    // Be conservative for init/update: they influence trip count
                    // heavily, so we currently leave them unchanged.
                    "init" | "update" => return,
                    "test" => {
                        // Apply a small, *integral* perturbation to the upper bound.
                        // For loops are almost always integer‑counted; snapping to
                        // integers keeps behavior predictable and avoids useless
                        // fractional bounds.
                        let mode = random_weighted_choice(
                            &mut self.rng,
                            &[
                                ("inc", 6),
                                ("dec", 4),
                                ("scale_down", 3),
                                ("scale_up", 1),
                                ("keep", 8),
                            ],
                        );
                        match mode {
                            "inc" => {
                                let step = self.rng.random_range(1i32..=5i32) as f64;
                                new_value = original + step;
                            }
                            "dec" => {
                                let step = self.rng.random_range(1i32..=5i32) as f64;
                                new_value = original - step;
                            }
                            "scale_down" => {
                                new_value = original * 0.5;
                            }
                            "scale_up" => {
                                new_value = original * 2.0;
                            }
                            "keep" | _ => {
                                new_value = original;
                            }
                        }

                        // Snap to nearest integer to avoid odd fractional loop
                        // bounds that rarely matter for coverage.
                        new_value = new_value.round();

                        new_value = self.clamp_for_test_bound(original, new_value);

                        num_lit.value = new_value;
                        let raw = self.format_number_raw(new_value);
                        num_lit.raw = Some(Atom::from(raw.as_str()));
                        return;
                    }
                    _ => {}
                }
            }

            // Numeric literals used as computed member indices, e.g. `arr[0]`.
            // Very large indices can cause the engine to allocate or search
            // huge sparse arrays and lead to timeouts, so strongly bias these
            // mutations towards small, non‑negative integers.
            if self.in_array_index {
                let choice = random_weighted_choice(
                    &mut self.rng,
                    &[
                        ("keep", 12),
                        ("small_delta", 10),
                        ("random_small", 8),
                        ("zero", 6),
                        ("one", 6),
                    ],
                );

                match choice {
                    "small_delta" => {
                        let delta = self.rng.random_range(-3i32..=3i32) as f64;
                        new_value = (original + delta).round();
                    }
                    "random_small" => {
                        new_value = self.rng.random_range(0i32..=32i32) as f64;
                    }
                    "zero" => {
                        new_value = 0.0;
                    }
                    "one" => {
                        new_value = 1.0;
                    }
                    "keep" | _ => {
                        new_value = original;
                    }
                }

                // Clamp to a safe index range and snap to integer.
                if new_value < 0.0 {
                    new_value = 0.0;
                }
                if new_value > Self::ARRAY_INDEX_MAX {
                    new_value = Self::ARRAY_INDEX_MAX;
                }
                new_value = new_value.round();

                num_lit.value = new_value;
                let raw = self.format_number_raw(new_value);
                num_lit.raw = Some(Atom::from(raw.as_str()));
                return;
            }

            // Non‑loop literals: weighted choice of mutation mode.
            // Bias towards small, local changes; keep extreme and
            // exceptional values at lower probability so they still
            // occur but don't dominate.
            let choice = random_weighted_choice(
                &mut self.rng,
                &[
                    ("small_delta", 18),
                    ("inc", 15),
                    ("dec", 15),
                    ("flip_sign", 10),
                    ("truncate_int", 8),
                    ("random_fraction", 8),
                    ("scale_mult", 8),
                    ("to_neg_zero", 5),
                    // ("pow2", 4),
                    ("to_extreme_large", 3),
                    ("to_extreme_small", 3),
                    ("to_nan", 2),
                    ("to_infinity", 2),
                    ("to_neg_infinity", 2),
                    ("to_undefined", 1),
                    ("to_null", 1),
                ],
            );

            match choice {
                "small_delta" => {
                    // For integer-valued literals, treat small_delta as a
                    // small *integral* adjustment to keep the value in the
                    // same "shape" while still exploring nearby values.
                    if self.rng.random_bool(0.7) {
                        let delta = small_delta(&mut self.rng, 1.0)
                            .round()
                            .clamp(-10.0, 10.0);
                        new_value = original + delta;
                    } else {
                        new_value += small_delta(&mut self.rng, original);
                    }
                }
                "inc" => {
                    let step = self.rng.random_range(1i32..=5i32) as f64;
                    new_value = original + step;
                }
                "dec" => {
                    let step = self.rng.random_range(1i32..=5i32) as f64;
                    new_value = original - step;
                }
                "turn_to_10k" => {
                    new_value = 10_000.0;
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
                    if self.rng.random_bool(0.1) {
                        new_value = -new_value;
                    }
                }
                "random_fraction" => {
                    new_value = self.rng.random_range(0.0f64..=1.0f64);
                }
                "truncate_int" => {
                    new_value = new_value.trunc();
                }
                "scale_mult" => {
                    let factor = if self.rng.random_bool(0.5) { 0.5 } else { 2.0 };
                    new_value *= factor;
                }
                _ => {}
            }

            // for now, I don't really care about this if
            // // Ensure we actually changed something
            // let changed = if new_value.is_nan() && !original.is_nan() {
            //     true
            // } else {
            //     new_value != original
            // };

            if choice == "undefined" || choice == "null" {
                num_lit.raw = Some(Atom::from(choice));
                return;
            }
            num_lit.value = new_value;
            let raw = self.format_number_raw(new_value);
            num_lit.raw = Some(Atom::from(raw.as_str()));
        }
    }
}

impl AstMutator for NumericTweaker {
    fn mutate(&self, mut ast: Script) -> Result<Script> {
        // println!("{:#?}", ast);
        let mut counter = CountNumericLiterals { count: 0 };
        ast.visit_with(&mut counter);
        if counter.count == 0 {
            // No numeric literals to mutate
            return Ok(ast);
        }

        // randomly choose a literal index to mutate
        let mut visitor = NumericTweakerVisitor::new(counter.count);
        ast.visit_mut_with(&mut visitor);

        // TODO: log telemetry about mutations

        Ok(ast)
    }
}

impl NumericTweaker {
    pub fn new() -> Self {
        Self
    }
}