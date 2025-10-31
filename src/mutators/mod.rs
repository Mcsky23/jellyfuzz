pub mod minifier;

use swc_ecma_visit::swc_ecma_ast::Script;

pub trait AstMutator {
    fn mutate(ast: Script) -> anyhow::Result<Script>;
}