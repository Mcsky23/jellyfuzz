use anyhow::Result;
use rand::Rng;
use rand::seq::IndexedRandom;

use crate::{code_generators::il::{BasicBlock, BlockId, BlockTerminator, FunctionIL, JellIL, ValueId}, mutators::js_objects::js_types::JsObjectType};

/// Generates random jellIL code.
pub struct CodeGenerator {
    pub rng: rand::rngs::ThreadRng,
    il: JellIL
}

/// Generates a random jellIL program.
impl CodeGenerator {
    pub fn new() -> Self {
        Self {
            rng: rand::thread_rng(),
            il: JellIL::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::parsing::parser::generate_js;

    use super::*;

    #[test]
    fn test_code_generator() {
        // let mut generator = CodeGenerator::new(None);
        // generator.generate_literal_declaration();
        // generator.generate_object_constructor();
        // generator.generate_instance_method_call();
        // generator.generate_instance_method_call();
        // generator.generate_static_method_call();
        // generator.generate_binary_expression();
        // let generated_ast = generator.ast;
        // // println!("{:#?}", generated_ast);

        // let source = generate_js(generated_ast).unwrap(); // unwrap like a boss
        // println!("Generated code:\n{}", String::from_utf8(source).unwrap());
    }
}
