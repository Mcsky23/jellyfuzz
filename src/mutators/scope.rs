use rand::seq::IndexedRandom;
use swc_ecma_visit::swc_ecma_ast::*;
use swc_ecma_visit::{VisitMut, VisitMutWith};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScopeKind {
    Global,
    Function,
    Block,
}

#[derive(Clone, Debug)]
pub struct ScopeRecord {
    kind: ScopeKind,
    exprs: Vec<Expr>,
    idents: Vec<Ident>,
    functions: Vec<Ident>,
}

impl ScopeRecord {
    fn new(kind: ScopeKind) -> Self {
        Self {
            kind,
            exprs: Vec::new(),
            idents: Vec::new(),
            functions: Vec::new(),
        }
    }
}

pub struct ScopeStack {
    scopes: Vec<ScopeRecord>,
}

impl ScopeStack {
    pub fn new() -> Self {
        Self {
            scopes: vec![ScopeRecord::new(ScopeKind::Global)],
        }
    }

    pub fn push_scope(&mut self, kind: ScopeKind) {
        self.scopes.push(ScopeRecord::new(kind));
    }

    pub fn pop_scope(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }

    fn current_scope_mut(&mut self) -> Option<&mut ScopeRecord> {
        self.scopes.last_mut()
    }

    fn first_scope_mut(&mut self) -> Option<&mut ScopeRecord> {
        self.scopes.first_mut()
    }

    fn find_hoist_scope_mut(&mut self) -> Option<&mut ScopeRecord> {
        self.scopes
            .iter_mut()
            .rev()
            .find(|scope| !matches!(scope.kind, ScopeKind::Block))
    }

    pub fn add_expr_candidate(&mut self, expr: Expr) {
        if let Some(scope) = self.current_scope_mut() {
            scope.exprs.push(expr);
        }
    }

    pub fn add_ident_to_current(&mut self, ident: Ident) {
        if let Some(scope) = self.current_scope_mut() {
            scope.idents.push(ident);
        }
    }

    pub fn extend_idents_on_current<I>(&mut self, idents: I)
    where
        I: IntoIterator<Item = Ident>,
    {
        if let Some(scope) = self.current_scope_mut() {
            scope.idents.extend(idents);
        }
    }

    pub fn add_ident_to_hoist(&mut self, ident: Ident) {
        if let Some(scope) = self.find_hoist_scope_mut() {
            scope.idents.push(ident);
            return;
        }

        if let Some(scope) = self.current_scope_mut() {
            scope.idents.push(ident);
        }
    }

    pub fn add_function_to_current(&mut self, ident: Ident) {
        if let Some(scope) = self.current_scope_mut() {
            scope.functions.push(ident);
        }
    }

    pub fn add_function_to_hoist(&mut self, ident: Ident) {
        if let Some(scope) = self.find_hoist_scope_mut() {
            scope.functions.push(ident);
            return;
        }

        if let Some(scope) = self.current_scope_mut() {
            scope.functions.push(ident);
        }
    }

    pub fn add_ident_to_global(&mut self, ident: Ident) {
        if let Some(scope) = self.first_scope_mut() {
            scope.idents.push(ident);
        }
    }

    pub fn collect_idents(&self) -> Vec<Ident> {
        self.scopes
            .iter()
            .rev()
            .flat_map(|scope| scope.idents.iter().cloned())
            .collect()
    }

    pub fn collect_idents_and_functions(&self) -> Vec<Ident> {
        self.scopes
            .iter()
            .rev()
            .flat_map(|scope| scope.idents.iter().chain(scope.functions.iter()).cloned())
            .collect()
    }

    pub fn collect_functions(&self) -> Vec<Ident> {
        self.scopes
            .iter()
            .rev()
            .flat_map(|scope| scope.functions.clone())
            .collect()
    }

    pub fn choose_expr(&self, rng: &mut rand::rngs::ThreadRng) -> Option<Expr> {
        // Choose from all visible scopes (innermost to outermost)
        let pool: Vec<Expr> = self
            .scopes
            .iter()
            .rev()
            .flat_map(|scope| scope.exprs.iter().cloned())
            .collect();

        pool.choose(rng).cloned()
    }
}

/// Recursively walk patterns and collect all binding identifiers.
pub fn collect_binding_idents_from_pat(pat: &Pat, out: &mut Vec<Ident>) {
    match pat {
        Pat::Ident(binding) => out.push(binding.id.clone()),
        Pat::Array(array_pat) => {
            for elem in &array_pat.elems {
                if let Some(elem_pat) = elem {
                    collect_binding_idents_from_pat(elem_pat, out);
                }
            }
        }
        Pat::Object(object_pat) => {
            for prop in &object_pat.props {
                match prop {
                    ObjectPatProp::KeyValue(kv) => {
                        collect_binding_idents_from_pat(&kv.value, out);
                    }
                    ObjectPatProp::Assign(assign) => {
                        out.push(assign.key.id.clone());
                    }
                    ObjectPatProp::Rest(rest) => {
                        collect_binding_idents_from_pat(&rest.arg, out);
                    }
                }
            }
        }
        Pat::Assign(assign_pat) => {
            collect_binding_idents_from_pat(&assign_pat.left, out);
        }
        Pat::Rest(rest_pat) => {
            collect_binding_idents_from_pat(&rest_pat.arg, out);
        }
        Pat::Expr(_) | Pat::Invalid(_) => {}
    }
}

pub fn extend_params_from_pats(scopes: &mut ScopeStack, params: &[Pat]) {
    let mut collected = Vec::new();
    for pat in params {
        collect_binding_idents_from_pat(pat, &mut collected);
    }
    scopes.extend_idents_on_current(collected);
}

pub fn extend_params_from_fn_params(scopes: &mut ScopeStack, params: &[Param]) {
    let mut collected = Vec::new();
    for param in params {
        collect_binding_idents_from_pat(&param.pat, &mut collected);
    }
    scopes.extend_idents_on_current(collected);
}

pub trait ScopedAstVisitor: VisitMut {
    fn scope_stack(&mut self) -> &mut ScopeStack;
    fn pending_function_names(&mut self) -> &mut Vec<Option<Ident>>;

    fn on_fn_decl_ident(&mut self, _ident: &Ident) {}
    fn on_fn_expr_ident(&mut self, _ident: &Option<Ident>) {}
    fn on_class_decl_ident(&mut self, _ident: &Ident) {}
    fn on_class_expr_ident(&mut self, _ident: &Ident) {}

    fn visit_block_stmt_scoped(&mut self, node: &mut BlockStmt) {
        {
            let scopes = self.scope_stack();
            scopes.push_scope(ScopeKind::Block);
        }
        node.visit_mut_children_with(self);
        self.scope_stack().pop_scope();
    }

    fn visit_arrow_expr_scoped(&mut self, node: &mut ArrowExpr) {
        {
            let scopes = self.scope_stack();
            scopes.push_scope(ScopeKind::Function);
            extend_params_from_pats(scopes, &node.params);
        }

        for pat in &mut node.params {
            pat.visit_mut_with(self);
        }

        match node.body.as_mut() {
            BlockStmtOrExpr::BlockStmt(block) => block.visit_mut_with(self),
            BlockStmtOrExpr::Expr(expr) => expr.visit_mut_with(self),
        }

        self.scope_stack().pop_scope();
    }

    fn visit_catch_clause_scoped(&mut self, node: &mut CatchClause) {
        {
            let scopes = self.scope_stack();
            scopes.push_scope(ScopeKind::Block);
            if let Some(param) = &node.param {
                let mut ids = Vec::new();
                collect_binding_idents_from_pat(param, &mut ids);
                scopes.extend_idents_on_current(ids);
            }
        }

        node.body.visit_mut_with(self);

        self.scope_stack().pop_scope();
    }

    fn visit_var_decl_scoped(&mut self, node: &mut VarDecl) {
        let mut names = Vec::new();
        for decl in &node.decls {
            collect_binding_idents_from_pat(&decl.name, &mut names);
        }

        if node.kind == VarDeclKind::Var {
            for ident in &names {
                self.scope_stack().add_ident_to_hoist(ident.clone());
            }
        }

        node.visit_mut_children_with(self);

        if matches!(node.kind, VarDeclKind::Let | VarDeclKind::Const) {
            self.scope_stack().extend_idents_on_current(names);
        }
    }

    fn visit_import_decl_scoped(&mut self, node: &mut ImportDecl) {
        node.visit_mut_children_with(self);

        for specifier in &node.specifiers {
            match specifier {
                ImportSpecifier::Named(named) => {
                    self.scope_stack().add_ident_to_global(named.local.clone());
                }
                ImportSpecifier::Default(default) => {
                    self.scope_stack()
                        .add_ident_to_global(default.local.clone());
                }
                ImportSpecifier::Namespace(ns) => {
                    self.scope_stack().add_ident_to_global(ns.local.clone());
                }
            }
        }
    }

    fn visit_fn_decl_scoped(&mut self, node: &mut FnDecl) {
        self.scope_stack().add_ident_to_hoist(node.ident.clone());
        self.on_fn_decl_ident(&node.ident);
        self.pending_function_names().push(Some(node.ident.clone()));
        node.function.visit_mut_with(self);
    }

    fn visit_fn_expr_scoped(&mut self, node: &mut FnExpr) {
        self.pending_function_names().push(node.ident.clone());
        self.on_fn_expr_ident(&node.ident);
        node.function.visit_mut_with(self);
    }

    fn visit_class_decl_scoped(&mut self, node: &mut ClassDecl) {
        self.scope_stack().add_ident_to_current(node.ident.clone());
        self.on_class_decl_ident(&node.ident);
        node.class.visit_mut_with(self);
    }

    fn visit_class_expr_scoped(&mut self, node: &mut ClassExpr) {
        if let Some(ident) = &node.ident {
            self.scope_stack().add_ident_to_current(ident.clone());
            self.on_class_expr_ident(ident);
        }
        node.class.visit_mut_with(self);
    }
}

macro_rules! scoped_visit_mut_methods {
    () => {
        fn visit_mut_block_stmt(&mut self, node: &mut BlockStmt) {
            self.visit_block_stmt_scoped(node);
        }

        fn visit_mut_arrow_expr(&mut self, node: &mut ArrowExpr) {
            self.visit_arrow_expr_scoped(node);
        }

        fn visit_mut_catch_clause(&mut self, node: &mut CatchClause) {
            self.visit_catch_clause_scoped(node);
        }

        fn visit_mut_var_decl(&mut self, node: &mut VarDecl) {
            self.visit_var_decl_scoped(node);
        }

        fn visit_mut_import_decl(&mut self, node: &mut ImportDecl) {
            self.visit_import_decl_scoped(node);
        }

        fn visit_mut_fn_decl(&mut self, node: &mut FnDecl) {
            self.visit_fn_decl_scoped(node);
        }

        fn visit_mut_fn_expr(&mut self, node: &mut FnExpr) {
            self.visit_fn_expr_scoped(node);
        }

        fn visit_mut_class_decl(&mut self, node: &mut ClassDecl) {
            self.visit_class_decl_scoped(node);
        }

        fn visit_mut_class_expr(&mut self, node: &mut ClassExpr) {
            self.visit_class_expr_scoped(node);
        }
    };
}
pub(crate) use scoped_visit_mut_methods;

/// Keeps track if we are inside a for statement and which part
/// (init, test, update) we are visiting.
macro_rules! for_stmt_visitor {
    (mut) => {
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
    };
    () => {
        fn visit_for_stmt(&mut self, node: &ForStmt) {
            let prev_in_for = self.in_for_stmt;
            if let Some(init) = &node.init {
                self.in_for_stmt = Some("init");
                init.visit_with(self);
            }
            if let Some(test) = &node.test {
                self.in_for_stmt = Some("test");
                test.visit_with(self);
            }
            if let Some(update) = &node.update {
                self.in_for_stmt = Some("update");
                update.visit_with(self);
            }
            self.in_for_stmt = None;
            node.body.visit_with(self);
            self.in_for_stmt = prev_in_for;
        }
    };
}
pub(crate) use for_stmt_visitor;
