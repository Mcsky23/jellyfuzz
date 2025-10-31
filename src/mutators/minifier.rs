pub struct Minifier;

use crate::mutators::AstMutator;
use swc_ecma_visit::swc_ecma_ast::Script;

impl AstMutator for Minifier {
    /// Rename variable names to v0, v1, v2, ...
    /// Comments are already removed when parsing the AST
    fn mutate(&self, ast: Script) -> anyhow::Result<Script> {
        
        unimplemented!();
        Ok(ast)
    }
}