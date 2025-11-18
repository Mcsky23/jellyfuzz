use anyhow::Result;
use rand::Rng;
use swc_ecma_visit::{Visit, VisitMut, VisitMutWith};
use swc_ecma_visit::{VisitWith, swc_ecma_ast::*};

use crate::mutators::AstMutator;
use crate::mutators::scope::{FuncRenamer, NameCollector, VarRenamer};

pub struct SpliceMutator;

struct StmtCollector {
    stmts: Vec<Stmt>,
}

impl Visit for StmtCollector {
    fn visit_stmt(&mut self, node: &Stmt) {
        self.stmts.push(node.clone());
        node.visit_children_with(self);
    }
}

impl StmtCollector {
    fn get_random_stmt_range(&self) -> Option<Vec<Stmt>> {
        if self.stmts.is_empty() {
            return None;
        }
        let mut rng = rand::rng();
        let start = rng.random_range(0..self.stmts.len());
        let end = rng.random_range(start + 1..=self.stmts.len());
        Some(self.stmts[start..end].to_vec())
    }
}

struct InsertStmtMutator {
    stmts_to_insert: Vec<Stmt>,
    insert_pos: usize,
    current_idx: usize,
    inserted: bool,
}

impl InsertStmtMutator {
    fn new(stmts_to_insert: Vec<Stmt>, insert_pos: usize) -> Self {
        Self {
            stmts_to_insert,
            insert_pos,
            current_idx: 0,
            inserted: false,
        }
    }

    fn try_insert_into_vec(&mut self, stmts: &mut Vec<Stmt>) {
        let mut i = 0;

        while i < stmts.len() {
            // Check for an insertion point before the current statement.
            if !self.inserted && self.current_idx == self.insert_pos {
                let to_insert = self.stmts_to_insert.clone();
                let insert_len = to_insert.len();
                for (offset, stmt) in to_insert.into_iter().enumerate() {
                    stmts.insert(i + offset, stmt);
                }

                self.inserted = true;
                // Skip over the newly inserted statements.
                i += insert_len;
                continue;
            }

            // Visit the original statement and then advance the global index.
            stmts[i].visit_mut_with(self);
            if !self.inserted {
                self.current_idx += 1;
            }
            i += 1;
        }

        // Possible insertion point at the end of this vector.
        if !self.inserted && self.current_idx == self.insert_pos {
            let pos = stmts.len();
            let to_insert = self.stmts_to_insert.clone();
            for stmt in to_insert {
                stmts.insert(pos, stmt);
            }
            self.inserted = true;
        }
    }
}

impl VisitMut for InsertStmtMutator {
    fn visit_mut_script(&mut self, n: &mut Script) {
        self.try_insert_into_vec(&mut n.body);
    }

    fn visit_mut_block_stmt(&mut self, n: &mut BlockStmt) {
        self.try_insert_into_vec(&mut n.stmts);
    }

    fn visit_mut_switch_case(&mut self, n: &mut SwitchCase) {
        self.try_insert_into_vec(&mut n.cons);
    }

    fn visit_mut_catch_clause(&mut self, n: &mut CatchClause) {
        self.try_insert_into_vec(&mut n.body.stmts);
    }
}

impl AstMutator for SpliceMutator {
    fn mutate(&self, _ast: Script) -> anyhow::Result<Script> {
        unreachable!("SpliceMutator does not support mutate; use splice instead");
    }

    fn splice(&self, ast: &Script, donor: &Script) -> Result<Script> {
        // first rename all variables and functions in both ASTs to avoid name collisions
        // for now we rename everything in donor AST, but renaming only what we splice could be better
        let mut donor = donor.clone();
        let mut collector = NameCollector::new();
        ast.visit_with(&mut collector);
        donor.visit_mut_with(&mut VarRenamer::new(collector.var_names.clone()));
        donor.visit_mut_with(&mut FuncRenamer::new(collector.func_names.clone()));

        let mut donor_collector = StmtCollector { stmts: Vec::new() };
        let mut collector = StmtCollector { stmts: Vec::new() };

        ast.visit_with(&mut collector);
        donor.visit_with(&mut donor_collector);

        let donor_stmts = match donor_collector.get_random_stmt_range() {
            Some(stmts) => stmts,
            None => return Ok(ast.clone()),
        };

        let insert_pos = {
            let mut rng = rand::rng();
            rng.gen_range(0..=collector.stmts.len())
        };

        let mut new_ast = ast.clone();
        let mut inserter = InsertStmtMutator::new(donor_stmts, insert_pos);
        new_ast.visit_mut_with(&mut inserter);
        Ok(new_ast)
    }
}

#[cfg(test)]
mod tests {
    use crate::parsing::parser::{generate_js, parse_js};

    use super::*;

    #[test]
    fn test_stmt_collector() {
        let script_path = "./test.js";
        let source = std::fs::read_to_string(script_path).expect("failed to readtest script");
        let ast = parse_js(source).expect("failed to parse test script");
        let mut collector = StmtCollector { stmts: Vec::new() };
        ast.visit_with(&mut collector);
        println!("Collected {} statements:", collector.stmts.len());
        for stmt in collector.stmts {
            println!("{:#?}", stmt);
            println!("-----------------------------------------------------------------------------------------------");
        }
    }

    #[test]
    fn test_splice_mutator() {
        let script_path1 = "./corpus_raw/test3.js";
        let source1 = std::fs::read_to_string(script_path1).expect("failed to read test script 1");
        let ast1 = parse_js(source1).expect("failed to parse test script 1");
        let script_path2 = "./corpus_raw/test3.js";
        let source2 = std::fs::read_to_string(script_path2).expect("failed to read test script 2");
        let ast2 = parse_js(source2).expect("failed to parse test script 2");

        let splice_mutator = SpliceMutator {};
        let spliced_ast = splice_mutator.splice(&ast1, &ast2).expect("splicing failed");
        let script = generate_js(spliced_ast).expect("failed to generate JS from spliced AST");
        println!("Spliced script: {}", String::from_utf8_lossy(&script));
    }
}
