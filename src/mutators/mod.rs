pub mod elements;
pub mod expressions;
pub mod literals;
pub mod minifier;
pub mod operators;
pub mod scope;
pub mod splice;
pub mod js_objects;

use std::sync::{Arc, Mutex};
use swc_ecma_visit::swc_ecma_ast::Script;
use rand::seq::{IndexedRandom, SliceRandom};

use crate::utils::rand_utils::random_weighted_choice;

pub trait AstMutator: Send + Sync {
    fn mutate(&self, ast: Script) -> anyhow::Result<Script>;
    fn splice(&self, _ast: &Script, _donor: &Script) -> anyhow::Result<Script> {
        Err(anyhow::anyhow!("splice not implemented for this mutator"))
    }
}

#[derive(Debug, Clone)]
pub struct MutatorStats {
    pub mean_reward: f64,
    pub total_reward: f64,
    pub uses: u64,
    pub last_reward: f64,
    pub invalid_count: u64,
    pub timeout_count: u64,
}

impl Default for MutatorStats {
    fn default() -> Self {
        Self {
            mean_reward: 0.0,
            total_reward: 0.0,
            uses: 0,
            last_reward: 0.0,
            invalid_count: 0,
            timeout_count: 0,
        }
    }
}

pub struct ManagedMutator {
    name: String,
    mutator: Box<dyn AstMutator>,
    stats: Mutex<MutatorStats>,
    splicer: bool,
}

impl ManagedMutator {
    pub fn new(name: impl Into<String>, mutator: Box<dyn AstMutator>, splicer: bool) -> Self {
        Self {
            name: name.into(),
            mutator,
            stats: Mutex::new(MutatorStats::default()),
            splicer,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn mutate(&self, ast: Script) -> anyhow::Result<Script> {
        // TODO: consider not locking everytime and changing this to an atomic update
        self.stats.lock().expect("mutator stats poisoned").uses += 1;
        self.mutator.mutate(ast)
    }

    pub fn splice(&self, ast: &Script, donor: &Script) -> anyhow::Result<Script> {
        self.stats.lock().expect("mutator stats poisoned").uses += 1;
        self.mutator.splice(ast, donor)
    }

    pub fn is_splicer(&self) -> bool {
        self.splicer
    }

    pub fn record_reward(&self, reward: f64) {
        let mut stats = self.stats.lock().expect("mutator stats poisoned");
        stats.uses += 1;
        stats.total_reward += reward;
        stats.mean_reward = if stats.uses == 0 {
            0.0
        } else {
            stats.total_reward / stats.uses as f64
        };
        stats.last_reward = reward;
    }

    pub fn record_invalid(&self, is_timeout: bool) {
        let mut stats = self.stats.lock().expect("mutator stats poisoned");
        stats.invalid_count += 1;
        if is_timeout {
            stats.timeout_count += 1;
        }
    }

    #[allow(dead_code)]
    pub fn stats_snapshot(&self) -> MutatorStats {
        self.stats.lock().expect("mutator stats poisoned").clone()
    }
}

pub fn get_ast_mutators() -> Vec<Arc<ManagedMutator>> {
    vec![
        Arc::new(ManagedMutator::new(
            "NumericTweaker",
            Box::new(literals::numeric_tweaker::NumericTweaker::new()),
            false,
        )),
        Arc::new(ManagedMutator::new(
            "BooleanFlipper",
            Box::new(literals::boolean_flipper::BooleanFlipper {}),
            false,
        )),
        Arc::new(ManagedMutator::new(
            "ArrayMutator",
            Box::new(literals::array_mutator::ArrayMutator {}),
            false,
        )),
        // Arc::new(ManagedMutator::new(
        //     "ConstructorCall",
        //     Box::new(literals::constructor_call::ConstructorCall {}),
        //     false,
        // )),
        Arc::new(ManagedMutator::new(
            "OperatorSwap",
            Box::new(operators::OperatorSwap {}),
            false,
        )),
        Arc::new(ManagedMutator::new(
            "ExpressionSwapDup",
            Box::new(expressions::ExpressionSwapDup {}),
            false,
        )),
        // Arc::new(ManagedMutator::new(
        //     "ElementAccessor",
        //     Box::new(elements::ElementAccessorMutator {}),
        //     false,
        // )),
        // Arc::new(ManagedMutator::new(
        //     "MethodCallMutator",
        //     Box::new(elements::MethodCallMutator {}),
        //     false,
        // )),
        Arc::new(ManagedMutator::new(
            "IdentSwapMutator",
            Box::new(expressions::IdentSwapMutator {}),
            false,
        )),
        Arc::new(ManagedMutator::new(
            "RemovePropMutator",
            Box::new(elements::RemovePropMutator {}),
            false,
        )),
        Arc::new(ManagedMutator::new(
            "SpliceMutator",
            Box::new(splice::SpliceMutator {}),
            true,
        )),
    ]
}

pub fn get_mutator_by_name(name: &str) -> Option<Arc<ManagedMutator>> {
    let mutators = get_ast_mutators();
    for m in mutators {
        if m.name() == name {
            return Some(m.clone());
        }
    }
    None
}

/// Returns a random mutator with weighted probabilities
/// Does NOT return splicers
pub fn get_weighted_ast_mutator_choice(
    mutators: &[Arc<ManagedMutator>],
) -> Arc<ManagedMutator> {
    let mut choices: Vec<(Arc<ManagedMutator>, f64)> = Vec::new();
    for m in mutators {
        if m.is_splicer() {
            continue;
        }
        let stats = m.stats_snapshot();
        let weight = if stats.uses == 0 {
            1.0
        } else {
            // stats.mean_reward + 1.0 / (stats.invalid_count as f64 + 1.0)s
            if stats.mean_reward > 0.0 {
                stats.mean_reward
            } else {
                0.1
            }
        };
        choices.push((m.clone(), weight));
    }
    random_weighted_choice(&mut rand::rng(), &choices)
}

// Returns a random splicer mutator
pub fn get_random_splicer(mutators: &[Arc<ManagedMutator>]) -> Option<Arc<ManagedMutator>> {
    let splicers: Vec<Arc<ManagedMutator>> = mutators
        .iter()
        .filter(|m| m.is_splicer())
        .cloned()
        .collect();
    
    if splicers.is_empty() {
        None
    } else {
        splicers.choose(&mut rand::rng()).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parsing::parser::*;
    use std::fs;

    #[tokio::test(flavor = "multi_thread")]
    async fn print_ast() {
        let script_path = "./test_out.js";
        let source = fs::read_to_string(script_path).expect("failed to read test script");
        let ast = parse_js(source).expect("failed to parse test script");
        println!("{:#?}", ast);
        let code = generate_js(ast)
            .unwrap();
        println!("code: {}", String::from_utf8(code).unwrap());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_mutator() {
        let script_path = "./test_out.js";
        let source = fs::read_to_string(script_path).expect("failed to read test script");
        let ast = parse_js(source.clone()).expect("failed to parse test script");
        // let minifier = Minifier;
        // let mutated_ast = minifier.mutate(ast).expect("minification failed");
        
        let mutator = get_mutator_by_name("ConstructorCall").expect("unknown mutator");
        let mutated_ast = mutator.mutate(ast).expect("mutation failed");
        let mutated_code = generate_js(mutated_ast).expect("code generation failed");

        println!("Original code:\n{}", source);
        println!("-----------------------------------");
        println!("Mutated code:\n{}", String::from_utf8_lossy(mutated_code.as_slice()));
    }
}