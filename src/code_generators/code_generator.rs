use anyhow::Result;
use rand::Rng;
use rand::seq::IndexedRandom;
use swc_atoms::Atom;
use swc_common::{DUMMY_SP, SyntaxContext};
use swc_ecma_visit::{Visit, VisitMut, VisitMutWith};
use swc_ecma_visit::{VisitWith, swc_ecma_ast::*};

use crate::code_generators::ast_helpers::{build_args, build_ctor_expr, build_property_call, build_var_decl};
use crate::mutators::js_objects::js_objects::{JsGlobalObject, get_random_global_object};
use crate::mutators::js_objects::js_types::JsObjectType;

/// Generates random code based on a couple of rules
pub struct CodeGenerator {
    rng: rand::rngs::ThreadRng,
    ast: Script,
    variable_name_counter: u32,
    scope_stack: Vec<Scope>,
}

struct Scope {
    // variables may have associated types (e.g. if they are instances of global objects)
    vars: Vec<VarInScope>,
    funcs: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct VarInScope {
    pub name: String,
    pub obj_type: Option<JsGlobalObject>,
}

impl CodeGenerator {
    pub fn new(base_ast: Option<Script>) -> Self {
        if base_ast.is_some() {
            todo!("Code generation with base AST is not implemented yet");
        }
        let base_ast = base_ast.unwrap_or(Script::default());
        CodeGenerator {
            rng: rand::rngs::ThreadRng::default(),
            ast: base_ast,
            variable_name_counter: 0,
            scope_stack: vec![
                Scope {
                    vars: vec![],
                    funcs: vec![],
                }
            ],
        }
    }

    fn scope_vars(&self) -> Vec<VarInScope> {
        self.scope_stack.last().unwrap().vars.clone()
    }

    fn add_scope_var(&mut self, var_name: String, obj_type: Option<JsGlobalObject>) {
        if let Some(scope) = self.scope_stack.last_mut() {
            scope.vars.push(VarInScope { name: var_name, obj_type });
        }
    }

    fn get_new_var_name(&mut self) -> String {
        let var_name = format!("v{}", self.variable_name_counter);
        self.variable_name_counter += 1;
        var_name
    }

    /// Generate a simple object constructor and add it to the AST
    /// Example: let v0 = new Array(5);
    fn generate_object_constructor(&mut self) {
        let obj_name = format!("v{}", self.variable_name_counter);
        self.variable_name_counter += 1;
        
        // choose a global object type randomly
        let global_obj = get_random_global_object(&mut self.rng);
        // choose a constructor signature randomly
        let ctor_signatures = global_obj.get_constructor_signatures();
        if ctor_signatures.is_empty() {
            unreachable!("Global object has no constructor signatures");
        }
        let ctor_signature = ctor_signatures
            .choose(&mut self.rng)
            .expect("should never panic because of the check above");

        println!("Chosen signature for {}: {:?}", global_obj.sym(), ctor_signature.types());

        // build argument expressions based on the constructor signature
        // println!("self.scope_vars(): {:?}", self.scope_vars());
        let args = build_args(&ctor_signature, &self.scope_vars());
        let ctor_expr = build_ctor_expr(&global_obj.sym(), args);
        
        let var_decl = build_var_decl(&obj_name, ctor_expr);
        self.add_scope_var(obj_name, Some(global_obj));
        self.ast.body.push(Stmt::Decl(Decl::Var(Box::new(var_decl))));
    }

    /// Generate an instance method call on a random variable in scope and store the result in a new variable
    /// Example: let v1 = v0.push(10);
    /// TODO: for now, always store return results in new variables, but we could also have void calls
    fn generate_instance_method_call(&mut self) {
        if self.scope_vars().is_empty() {
            return;
        }
        let vars_in_scope = self.scope_vars();
        let var_in_scope = vars_in_scope
            .choose(&mut self.rng)
            .expect("should never panic because of the check above");

        // there are two cases here: the variable is of a known object type, or unknown
        if let Some(obj_type) = &var_in_scope.obj_type {
            // choose a method randomly and build a call expression
            let method = obj_type.methods()
                .choose(&mut self.rng)
                .expect("should never panic because global object has methods");
            let method_signature = method.signatures()
                .choose(&mut self.rng)
                .expect("should never panic because method has signatures");
            let args = build_args(&method_signature, &self.scope_vars());
            let call_expr = build_property_call(&var_in_scope.name, &method.sym(), args);

            let new_var_name = self.get_new_var_name();
            let var_decl = build_var_decl(&new_var_name, call_expr);
            self.add_scope_var(new_var_name, None);

            self.ast.body.push(Stmt::Decl(Decl::Var(Box::new(var_decl))));
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::parsing::parser::generate_js;

    use super::*;

    #[test]
    fn test_code_generator() {
        let mut generator = CodeGenerator::new(None);
        generator.generate_object_constructor();
        generator.generate_instance_method_call();
        generator.generate_instance_method_call();
        let generated_ast = generator.ast;
        // println!("{:#?}", generated_ast);

        let source = generate_js(generated_ast).unwrap(); // unwrap like a boss
        println!("Generated code:\n{}", String::from_utf8(source).unwrap());
    }
}
