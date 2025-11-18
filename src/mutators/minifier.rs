use std::collections::{HashMap, HashSet};
use swc_ecma_visit::swc_ecma_ast::*;
use swc_ecma_visit::{VisitMut, VisitMutWith};

use crate::mutators::scope::{FuncRenamer, VarRenamer};

/// This minifier renames variables and functions to shorter names
/// Logic is the following: for each function, rename all variables
/// inside the function, then call function renamer inside that
/// function and so on.
///
/// TODO: handle classes and class methods

pub struct Minifier;


impl Minifier {
    /// Rename variable names to v0, v1, v2, ...
    /// Comments are already removed when parsing the AST
    pub fn mutate(&self, mut ast: Script) -> anyhow::Result<Script> {
        let mut var_visitor = VarRenamer::new(HashSet::new());
        ast.visit_mut_with(&mut var_visitor);
        let mut func_visitor = FuncRenamer::new(HashSet::new());
        ast.visit_mut_with(&mut func_visitor);
        Ok(ast)
    }
}
