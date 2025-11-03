pub mod expressions;
pub mod literals;
pub mod minifier;
pub mod operators;

use std::sync::{Arc, Mutex};

use swc_ecma_visit::swc_ecma_ast::Script;

pub trait AstMutator: Send + Sync {
    fn mutate(&self, ast: Script) -> anyhow::Result<Script>;
}

#[derive(Debug, Clone)]
pub struct MutatorStats {
    pub mean_reward: f64,
    pub total_reward: f64,
    pub uses: u64,
    pub last_reward: f64,
    pub invalid_count: u64,
}

impl Default for MutatorStats {
    fn default() -> Self {
        Self {
            mean_reward: 0.0,
            total_reward: 0.0,
            uses: 0,
            last_reward: 0.0,
            invalid_count: 0,
        }
    }
}

pub struct ManagedMutator {
    name: String,
    mutator: Box<dyn AstMutator>,
    stats: Mutex<MutatorStats>,
}

impl ManagedMutator {
    pub fn new(name: impl Into<String>, mutator: Box<dyn AstMutator>) -> Self {
        Self {
            name: name.into(),
            mutator,
            stats: Mutex::new(MutatorStats::default()),
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

    pub fn record_invalid(&self) {
        let mut stats = self.stats.lock().expect("mutator stats poisoned");
        stats.invalid_count += 1;
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
            Box::new(literals::NumericTweaker::new()),
        )),
        Arc::new(ManagedMutator::new(
            "BooleanFlipper",
            Box::new(literals::BooleanFlipper {}),
        )),
        Arc::new(ManagedMutator::new(
            "ArrayLengthMutator",
            Box::new(literals::ArrayLengthMutator {}),
        )),
        Arc::new(ManagedMutator::new(
            "OperatorSwap",
            Box::new(operators::OperatorSwap {}),
        )),
        Arc::new(ManagedMutator::new(
            "ExpressionSwap",
            Box::new(expressions::ExpressionSwap {}),
        )),
        Arc::new(ManagedMutator::new(
            "ExpressionDup",
            Box::new(expressions::ExpressionDup {}),
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
