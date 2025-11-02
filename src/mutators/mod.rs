pub mod literals;
pub mod minifier;

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
}

impl Default for MutatorStats {
    fn default() -> Self {
        Self {
            mean_reward: 0.0,
            total_reward: 0.0,
            uses: 0,
            last_reward: 0.0,
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

    #[allow(dead_code)]
    pub fn stats_snapshot(&self) -> MutatorStats {
        self.stats.lock().expect("mutator stats poisoned").clone()
    }
}

pub fn get_ast_mutators() -> Vec<Arc<ManagedMutator>> {
    vec![Arc::new(ManagedMutator::new(
        "numeric_tweaker",
        Box::new(literals::NumericTweaker::new()),
    ))]
}
