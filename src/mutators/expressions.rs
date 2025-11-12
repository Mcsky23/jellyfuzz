use std::process::exit;

use rand::Rng;
use rand::seq::IndexedRandom;
use swc_common::SyntaxContext;
use swc_ecma_visit::{Visit, VisitMut, VisitMutWith};
use swc_ecma_visit::{VisitWith, swc_ecma_ast::*};

use crate::mutators::AstMutator;
use crate::mutators::scope::{
    ScopeKind, ScopeStack, ScopedAstVisitor, extend_params_from_fn_params, for_stmt_visitor, scoped_visit_mut_methods
};

const FUNCTION_REPLACEMENT_PROBABILITY: f64 = 0.1;

pub struct ExpressionSwapDup;

struct ExpressionListCollector {
    exprs: Vec<Expr>,
    in_for_stmt: Option<&'static str>,
}

impl Default for ExpressionListCollector {
    fn default() -> Self {
        Self { exprs: Vec::new(), in_for_stmt: None }
    }
}

impl Visit for ExpressionListCollector {
    for_stmt_visitor!();

    fn visit_expr(&mut self, node: &Expr) {
        // for now, avoid collecting expressions inside for statements
        if self.in_for_stmt.is_none() {
            self.exprs.push(node.clone());
            node.visit_children_with(self);
        }
    }
}

enum SwapDupMode {
    Swap(SwapState),
    Dup(DupState),
}

struct SwapState {
    idx1: usize,
    idx2: usize,
    exprs: Vec<Expr>,
}

struct DupState {
    idx_to_replace: usize,
    replaced: bool,
}

enum Mode {
    Swap,
    Dup,
}

struct ExpressionSwapDupVisitor {
    rng: rand::rngs::ThreadRng,
    mode: Mode,
    current_idx: usize,
    swap_state: Option<SwapState>,
    dup_state: Option<DupState>,
    scopes: ScopeStack,
    pending_function_names: Vec<Option<Ident>>,
    in_for_stmt: Option<&'static str>,
}

impl ScopedAstVisitor for ExpressionSwapDupVisitor {
    fn scope_stack(&mut self) -> &mut ScopeStack {
        &mut self.scopes
    }

    fn pending_function_names(&mut self) -> &mut Vec<Option<Ident>> {
        &mut self.pending_function_names
    }

    fn on_fn_decl_ident(&mut self, ident: &Ident) {
        self.scopes.add_function_to_hoist(ident.clone());
    }
}

impl ExpressionSwapDupVisitor {
    fn new(rng: rand::rngs::ThreadRng, mode: SwapDupMode) -> Self {
        let (mode_flag, swap_state, dup_state) = match mode {
            SwapDupMode::Swap(state) => (Mode::Swap, Some(state), None),
            SwapDupMode::Dup(state) => (Mode::Dup, None, Some(state)),
        };
        Self {
            rng,
            mode: mode_flag,
            current_idx: 0,
            swap_state,
            dup_state,
            scopes: ScopeStack::new(),
            pending_function_names: Vec::new(),
            in_for_stmt: None
        }
    }

    fn maybe_pick_function_reference(&mut self) -> Option<Expr> {
        if !self.rng.random_bool(FUNCTION_REPLACEMENT_PROBABILITY) {
            return None;
        }

        let candidates = self.scopes.collect_functions();

        if candidates.is_empty() {
            return None;
        }

        candidates.choose(&mut self.rng).cloned().map(Expr::Ident)
    }

    fn pick_replacement(&mut self) -> Option<Expr> {
        if let Some(expr) = self.maybe_pick_function_reference() {
            return Some(expr);
        }

        if self.rng.random_bool(0.5) {
            let candidates: Vec<Expr> = self
                .scopes
                .collect_idents_and_functions()
                .into_iter()
                .map(Expr::Ident)
                .collect();
            if candidates.is_empty() {
                return None;
            }
            return candidates.choose(&mut self.rng).cloned();
        }

        self.scopes.choose_expr(&mut self.rng)
    }
}

impl VisitMut for ExpressionSwapDupVisitor {
    scoped_visit_mut_methods!();
    for_stmt_visitor!(mut);

    fn visit_mut_expr(&mut self, node: &mut Expr) {

        if self.in_for_stmt.is_some() {
            node.visit_mut_children_with(self);
            return;
        }
        
        match self.mode {
            Mode::Swap => {
                let idx = self.current_idx;
                self.current_idx += 1;

                let (idx1, idx2, fallback_for_idx1, fallback_for_idx2) =
                    if let Some(state) = self.swap_state.as_ref() {
                        (
                            Some(state.idx1),
                            Some(state.idx2),
                            Some(state.exprs[state.idx2].clone()),
                            Some(state.exprs[state.idx1].clone()),
                        )
                    } else {
                        (None, None, None, None)
                    };

                node.visit_mut_children_with(self);

                if let (Some(target_idx), Some(fallback)) = (idx1, fallback_for_idx1.as_ref()) {
                    if idx == target_idx {
                        let replacement = self
                            .maybe_pick_function_reference()
                            .unwrap_or_else(|| fallback.clone());
                        *node = replacement;
                    }
                }

                if let (Some(target_idx), Some(fallback)) = (idx2, fallback_for_idx2.as_ref()) {
                    if idx == target_idx {
                        let replacement = self
                            .maybe_pick_function_reference()
                            .unwrap_or_else(|| fallback.clone());
                        *node = replacement;
                    }
                }

                self.scopes.add_expr_candidate(node.clone());
            }
            Mode::Dup => {
                let (idx_to_replace, already_replaced) =
                    if let Some(state) = self.dup_state.as_ref() {
                        (state.idx_to_replace, state.replaced)
                    } else {
                        (usize::MAX, false)
                    };

                let idx = self.current_idx;
                self.current_idx += 1;

                if already_replaced {
                    node.visit_mut_children_with(self);
                    self.scopes.add_expr_candidate(node.clone());
                    return;
                }

                let mut did_replace = false;
                if idx == idx_to_replace {
                    if let Some(mut replacement) = self.pick_replacement() {
                        let candidates = self.scopes.collect_idents_and_functions();
                        swap_idents_in_expr(&mut replacement, &mut self.rng, &candidates);
                        *node = replacement;
                        did_replace = true;
                    }
                }

                node.visit_mut_children_with(self);
                self.scopes.add_expr_candidate(node.clone());

                if did_replace {
                    if let Some(state) = self.dup_state.as_mut() {
                        state.replaced = true;
                    }
                }
            }
        }
    }

    fn visit_mut_function(&mut self, node: &mut Function) {
        let fn_name = self.pending_function_names.pop().flatten();

        self.scopes.push_scope(ScopeKind::Function);

        if let Some(name) = fn_name.clone() {
            self.scopes.add_ident_to_current(name.clone());
            self.scopes.add_function_to_current(name);
        }

        extend_params_from_fn_params(&mut self.scopes, &node.params);

        for param in &mut node.params {
            param.visit_mut_with(self);
        }

        if let Some(body) = &mut node.body {
            body.visit_mut_with(self);
        }

        self.scopes.pop_scope();
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

impl AstMutator for ExpressionSwapDup {
    fn mutate(&self, mut ast: Script) -> anyhow::Result<Script> {
        // println!("{:#?}", ast);
        // std::process::exit(0);

        let mut collector = ExpressionListCollector::default();
        ast.visit_with(&mut collector);
        let expr_count = collector.exprs.len();
        if expr_count < 2 {
            return Ok(ast);
        }

        let mut rng = rand::rng();
        let use_swap = rng.random_bool(0.1);

        let mode = if use_swap {
            let idx1 = rng.random_range(0..expr_count);
            let mut idx2 = rng.random_range(0..expr_count);
            while idx2 == idx1 {
                idx2 = rng.random_range(0..expr_count);
            }
            SwapDupMode::Swap(SwapState {
                idx1,
                idx2,
                exprs: collector.exprs,
            })
        } else {
            let idx_to_replace = rng.random_range(0..expr_count);
            SwapDupMode::Dup(DupState {
                idx_to_replace,
                replaced: false,
            })
        };

        let mut visitor = ExpressionSwapDupVisitor::new(rng, mode);
        ast.visit_mut_with(&mut visitor);
        Ok(ast)
    }
}
