use std::collections::HashMap;
use swc_ecma_visit::swc_ecma_ast::*;
use swc_ecma_visit::{VisitMut, VisitMutWith};

/// This minifier renames variables and functions to shorter names
/// Logic is the following: for each function, rename all variables
/// inside the function, then call function renamer inside that
/// function and so on.
///
/// TODO: handle classes and class methods

pub struct Minifier;
pub struct VarRenamer {
    var_count: usize,
    scope_stack: Vec<HashMap<String, String>>,
}

pub struct FuncRenamer {
    func_count: usize,
    func_rename_map: HashMap<String, String>,
}

impl VarRenamer {
    pub fn new() -> Self {
        Self {
            var_count: 0,
            scope_stack: vec![HashMap::new()],
        }
    }

    pub fn next_var_name(&mut self) -> String {
        let name = format!("v{}", self.var_count);
        self.var_count += 1;
        name
    }

    fn resolve_new_name(&mut self, orig: &str) -> String {
        if let Some(existing) = self.lookup_name(orig) {
            return existing;
        }

        let name = self.next_var_name();
        if let Some(scope) = self.scope_stack.last_mut() {
            scope.insert(orig.to_string(), name.clone());
        }
        name
    }

    fn lookup_name(&self, orig: &str) -> Option<String> {
        self.scope_stack
            .iter()
            .rev()
            .find_map(|scope| scope.get(orig).cloned())
    }

    fn push_scope(&mut self) {
        self.scope_stack.push(HashMap::new());
    }

    fn pop_scope(&mut self) {
        self.scope_stack.pop();
    }
}

impl FuncRenamer {
    pub fn new() -> Self {
        Self {
            func_count: 0,
            func_rename_map: HashMap::new(),
        }
    }

    pub fn next_func_name(&mut self) -> String {
        let name = format!("f{}", self.func_count);
        self.func_count += 1;
        name
    }
}

impl VisitMut for VarRenamer {
    fn visit_mut_function(&mut self, node: &mut Function) {
        self.push_scope();
        node.visit_mut_children_with(self);
        self.pop_scope();
    }

    fn visit_mut_binding_ident(&mut self, node: &mut BindingIdent) {
        if let Some(type_ann) = &mut node.type_ann {
            type_ann.visit_mut_with(self);
        }

        let orig = node.id.sym.to_string();
        let new_name = self.resolve_new_name(&orig);
        node.id.sym = new_name.into();
    }

    fn visit_mut_ident(&mut self, node: &mut Ident) {
        if let Some(new_name) = self.lookup_name(&node.sym.to_string()) {
            node.sym = new_name.clone().into();
        }
    }
}

impl VisitMut for FuncRenamer {
    fn visit_mut_fn_decl(&mut self, node: &mut FnDecl) {
        let orig = node.ident.sym.to_string();
        let new_name = if self.func_rename_map.contains_key(&orig) {
            self.func_rename_map.get(&orig).unwrap().clone()
        } else {
            let name = self.next_func_name();
            self.func_rename_map.insert(orig.clone(), name.clone());
            name
        };
        node.ident.sym = new_name.into();

        node.function.visit_mut_with(self);
    }

    fn visit_mut_call_expr(&mut self, node: &mut CallExpr) {
        // node.visit_mut_children_with(self);
        for arg in node.args.iter_mut() {
            arg.visit_mut_with(self);
        }
        if let Some(expr) = node.callee.as_mut_expr() {
            if let Expr::Ident(ident) = &mut **expr {
                let orig = ident.sym.to_string();
                if let Some(new_name) = self.func_rename_map.get(&orig) {
                    ident.sym = new_name.clone().into();
                }
            }
        }
    }
}

impl Minifier {
    /// Rename variable names to v0, v1, v2, ...
    /// Comments are already removed when parsing the AST
    pub fn mutate(&self, mut ast: Script) -> anyhow::Result<Script> {
        let mut var_visitor = VarRenamer::new();
        ast.visit_mut_with(&mut var_visitor);
        let mut func_visitor = FuncRenamer::new();
        ast.visit_mut_with(&mut func_visitor);
        Ok(ast)
    }
}
