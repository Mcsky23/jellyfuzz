use anyhow::Result;
use rand::Rng;
use rand::seq::IndexedRandom;
use swc_atoms::Atom;
use swc_common::SyntaxContext;
use swc_ecma_visit::{Visit, VisitMut, VisitMutWith};
use swc_ecma_visit::{VisitWith, swc_ecma_ast::*};

use crate::mutators::AstMutator;
use crate::utils::rand_utils::{random_weighted_choice, small_delta};

/// ExpressionSwapper
/// This mutator chooses 2 random Expr in the AST and swaps them.
pub struct ExpressionSwap;
pub struct ExpressionSwapVisitor {
    rng: rand::rngs::ThreadRng,
    idx_to_mutate1: usize,
    idx_to_mutate2: usize,
    current_idx: usize,
    exprs: Vec<Expr>,
}
pub struct ExpressionCollector {
    pub exprs: Vec<Expr>,
}

impl Visit for ExpressionCollector {
    fn visit_expr(&mut self, node: &Expr) {
        self.exprs.push(node.clone());
        // println!("Collected expr: {:?}", node);
        node.visit_children_with(self);
    }
}

// Naive implementation for now(or is it?).
// Key takeaways(TODO):
// - check that expr2 exists in the scope of expr1

impl VisitMut for ExpressionSwapVisitor {
    fn visit_mut_expr(&mut self, node: &mut Expr) {
        node.visit_mut_children_with(self);
        if self.current_idx == self.idx_to_mutate1 {
            let expr2 = self.exprs[self.idx_to_mutate2].clone();
            *node = expr2;
        } else if self.current_idx == self.idx_to_mutate2 {
            let expr1 = self.exprs[self.idx_to_mutate1].clone();
            *node = expr1;
        }
        self.current_idx += 1;
    }
}

impl AstMutator for ExpressionSwap {
    fn mutate(&self, mut ast: Script) -> anyhow::Result<Script> {
        let mut collector = ExpressionCollector { exprs: Vec::new() };
        ast.visit_with(&mut collector);
        let expr_count = collector.exprs.len();
        if expr_count < 2 {
            return Ok(ast);
        }

        let mut rng = rand::rng();
        let idx1 = rng.random_range(0..expr_count);
        let mut idx2 = rng.random_range(0..expr_count);
        while idx2 == idx1 {
            idx2 = rng.random_range(0..expr_count);
        }

        let mut visitor = ExpressionSwapVisitor {
            rng,
            idx_to_mutate1: idx1,
            idx_to_mutate2: idx2,
            current_idx: 0,
            exprs: collector.exprs,
        };
        ast.visit_mut_with(&mut visitor);
        Ok(ast)
    }
}

// ====================================================================================================
//
/// ExpressionDuplicator
/// This mutator is kind of like ExpressionSwapper, but instead of swapping
/// two expressions, it duplicates one expression and replaces another with it.
/// Choose a random index which will be replaced and when we reach it(in the AST traversal),
/// choose a random expression in the current scope to duplicate and replace the target with it.
/// 
/// TODO: Implement a base trait for scope management
pub struct ExpressionDup;
pub struct ExpressionDupVisitor {
    rng: rand::rngs::ThreadRng,
    idx_to_replace: usize,
    current_idx: usize,
    replaced: bool,
    scopes: Vec<Scope>,
    pending_function_names: Vec<Option<Ident>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ScopeKind {
    Global,
    Function,
    Block,
}

#[derive(Clone, Debug)]
struct Scope {
    kind: ScopeKind,
    exprs: Vec<Expr>,
    idents: Vec<Ident>,
}

impl Scope {
    fn new(kind: ScopeKind) -> Self {
        Self {
            kind,
            exprs: Vec::new(),
            idents: Vec::new(),
        }
    }
}

impl ExpressionDupVisitor {
    fn push_scope(&mut self, kind: ScopeKind) {
        self.scopes.push(Scope::new(kind));
    }

    fn pop_scope(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }

    fn add_expr_candidate(&mut self, expr: Expr) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.exprs.push(expr);
        }
    }

    fn add_binding_to_current_scope(&mut self, ident: Ident) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.idents.push(ident);
        }
    }

    fn add_binding_to_hoist_scope(&mut self, ident: Ident) {
        if let Some(scope) = self
            .scopes
            .iter_mut()
            .rev()
            .find(|scope| !matches!(scope.kind, ScopeKind::Block))
        {
            scope.idents.push(ident);
            return;
        }

        if let Some(scope) = self.scopes.last_mut() {
            scope.idents.push(ident);
        }
    }

    fn collect_available_idents(&self) -> Vec<Ident> {
        self.scopes
            .iter()
            .flat_map(|scope| scope.idents.clone())
            .collect()
    }

    fn pick_replacement(&mut self) -> Option<Expr> {
        for scope in self.scopes.iter().rev() {
            if let Some(expr) = scope.exprs.choose(&mut self.rng).cloned() {
                return Some(expr);
            }
        }
        None
    }
}

/// Recursively walk patterns and collect all binding identifiers
fn collect_binding_idents_from_pat(pat: &Pat, out: &mut Vec<Ident>) {
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

/// Swap identifiers in the given expression with other compatible identifiers from the candidates list
fn swap_idents_in_expr(expr: &mut Expr, rng: &mut rand::rngs::ThreadRng, candidates: &[Ident]) {
    if candidates.len() < 2 {
        return;
    }

    struct IdentRewriter<'a> {
        rng: &'a mut rand::rngs::ThreadRng,
        candidates: &'a [Ident],
    }

    impl VisitMut for IdentRewriter<'_> {
        fn visit_mut_expr(&mut self, node: &mut Expr) {
            match node {
                Expr::Ident(ident) => {
                    let ident_ctxt = ident.ctxt;
                    let compatible: Vec<&Ident> = self
                        .candidates
                        .iter()
                        .filter(|cand| {
                            cand.sym != ident.sym
                                && (cand.ctxt == ident_ctxt
                                    || cand.ctxt == SyntaxContext::empty()
                                    || ident_ctxt == SyntaxContext::empty())
                        })
                        .collect();

                    if compatible.is_empty() || !self.rng.random_bool(0.5) {
                        return;
                    }

                    if let Some(replacement) = compatible.choose(self.rng) {
                        ident.sym = replacement.sym.clone();
                        ident.ctxt = replacement.ctxt;
                    }
                }
                _ => node.visit_mut_children_with(self),
            }
        }
    }

    expr.visit_mut_with(&mut IdentRewriter { rng, candidates });
}

impl VisitMut for ExpressionDupVisitor {
    fn visit_mut_expr(&mut self, node: &mut Expr) {
        let current = self.current_idx;
        self.current_idx += 1;

        if self.replaced {
            node.visit_mut_children_with(self);
            return;
        }

        if current == self.idx_to_replace {
            if let Some(mut replacement) = self.pick_replacement() {
                let candidates = self.collect_available_idents();
                swap_idents_in_expr(&mut replacement, &mut self.rng, &candidates);
                *node = replacement;
                self.replaced = true;
            }
        }

        node.visit_mut_children_with(self);

        self.add_expr_candidate(node.clone());
    }

    fn visit_mut_block_stmt(&mut self, node: &mut BlockStmt) {
        self.push_scope(ScopeKind::Block);
        node.visit_mut_children_with(self);
        self.pop_scope();
    }

    fn visit_mut_arrow_expr(&mut self, node: &mut ArrowExpr) {
        self.push_scope(ScopeKind::Function);

        let mut params = Vec::new();
        for pat in &node.params {
            collect_binding_idents_from_pat(pat, &mut params);
        }
        if let Some(scope) = self.scopes.last_mut() {
            scope.idents.extend(params);
        }

        for pat in &mut node.params {
            pat.visit_mut_with(self);
        }

        match node.body.as_mut() {
            BlockStmtOrExpr::BlockStmt(block) => block.visit_mut_with(self),
            BlockStmtOrExpr::Expr(expr) => expr.visit_mut_with(self),
        }

        self.pop_scope();
    }

    fn visit_mut_function(&mut self, node: &mut Function) {
        let fn_name = self.pending_function_names.pop().flatten();

        self.push_scope(ScopeKind::Function);

        if let Some(name) = fn_name.clone() {
            self.add_binding_to_current_scope(name);
        }

        let mut params = Vec::new();
        for param in &node.params {
            collect_binding_idents_from_pat(&param.pat, &mut params);
        }
        if let Some(scope) = self.scopes.last_mut() {
            scope.idents.extend(params);
        }

        for param in &mut node.params {
            param.visit_mut_with(self);
        }

        if let Some(body) = &mut node.body {
            body.visit_mut_with(self);
        }

        self.pop_scope();
    }

    fn visit_mut_fn_decl(&mut self, node: &mut FnDecl) {
        self.add_binding_to_hoist_scope(node.ident.clone());
        self.pending_function_names.push(Some(node.ident.clone()));
        node.function.visit_mut_with(self);
    }

    fn visit_mut_fn_expr(&mut self, node: &mut FnExpr) {
        self.pending_function_names.push(node.ident.clone());
        node.function.visit_mut_with(self);
    }

    fn visit_mut_class_decl(&mut self, node: &mut ClassDecl) {
        self.add_binding_to_current_scope(node.ident.clone());
        node.class.visit_mut_with(self);
    }

    fn visit_mut_class_expr(&mut self, node: &mut ClassExpr) {
        if let Some(ident) = &node.ident {
            self.add_binding_to_current_scope(ident.clone());
        }
        node.class.visit_mut_with(self);
    }

    fn visit_mut_catch_clause(&mut self, node: &mut CatchClause) {
        self.push_scope(ScopeKind::Block);

        if let Some(param) = &node.param {
            let mut ids = Vec::new();
            collect_binding_idents_from_pat(param, &mut ids);
            if let Some(scope) = self.scopes.last_mut() {
                scope.idents.extend(ids);
            }
        }

        node.body.visit_mut_with(self);

        self.pop_scope();
    }

    fn visit_mut_var_decl(&mut self, node: &mut VarDecl) {
        let mut names = Vec::new();
        for decl in &node.decls {
            collect_binding_idents_from_pat(&decl.name, &mut names);
        }

        if node.kind == VarDeclKind::Var {
            for ident in &names {
                self.add_binding_to_hoist_scope(ident.clone());
            }
        }

        node.visit_mut_children_with(self);

        match node.kind {
            VarDeclKind::Var => {}
            VarDeclKind::Let | VarDeclKind::Const => {
                if let Some(scope) = self.scopes.last_mut() {
                    scope.idents.extend(names);
                }
            }
        }
    }

    fn visit_mut_import_decl(&mut self, node: &mut ImportDecl) {
        node.visit_mut_children_with(self);

        if let Some(scope) = self.scopes.first_mut() {
            for specifier in &node.specifiers {
                match specifier {
                    ImportSpecifier::Named(named) => {
                        scope.idents.push(named.local.clone());
                    }
                    ImportSpecifier::Default(default) => {
                        scope.idents.push(default.local.clone());
                    }
                    ImportSpecifier::Namespace(ns) => {
                        scope.idents.push(ns.local.clone());
                    }
                }
            }
        }
    }
}

impl AstMutator for ExpressionDup {
    fn mutate(&self, mut ast: Script) -> anyhow::Result<Script> {
        let mut collector = ExpressionCollector { exprs: Vec::new() };
        ast.visit_with(&mut collector);
        let expr_count = collector.exprs.len();
        if expr_count < 2 {
            return Ok(ast);
        }

        let mut rng = rand::rng();
        let idx_to_replace = rng.random_range(0..expr_count);

        let mut visitor = ExpressionDupVisitor {
            rng,
            idx_to_replace,
            current_idx: 0,
            replaced: false,
            scopes: vec![Scope::new(ScopeKind::Global)],
            pending_function_names: Vec::new(),
        };
        ast.visit_mut_with(&mut visitor);
        Ok(ast)
    }
}
