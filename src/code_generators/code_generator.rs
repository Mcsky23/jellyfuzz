use anyhow::Result;
use rand::Rng;
use rand::seq::IndexedRandom;

use crate::{code_generators::il::{BasicBlock, BlockBuilder, BlockId, BlockTerminator, FunctionIL, FunctionILBuilder, JellIL, ValueId}, mutators::js_objects::{js_objects::{JsGlobalObject, get_random_global_object}, js_types::JsObjectType}};
use crate::code_generators::il::*;

/// Generates random jellIL code.
pub struct CodeGenerator {
    pub rng: rand::rngs::ThreadRng,
    il: JellIL
}

pub struct BlockGenerator<'a> {
    pub rng: rand::rngs::ThreadRng,
    builder: BlockBuilder<'a>,
}

impl<'a> BlockGenerator<'a> {
    pub fn new(rng: rand::rngs::ThreadRng, func_builder: &'a mut FunctionILBuilder) -> Self {
        // TODO: for now, empty args. Later, pass in as args all values in scope.
        let block_builder = func_builder.new_block_builder(vec![]);
        Self {
            rng,
            builder: block_builder,
        }
    }

    pub fn random_block(mut self, budget: usize) -> Result<BlockId> {
        for _ in 0..budget {
            // randomly choose an instruction to generate
            let instr_choice = self.rng.random_range(0..2);
            match instr_choice {
                0 => { self.gen_load_literal(); },
                1 => { self.gen_binary_operation(); },
                _ => unreachable!(),
            }
        }

        self.builder.set_terminator(
            BlockTerminator::Return(None)
        );

        Ok(self.builder.finish())
    }

    // ============================ Instruction Generators ============================

    /// Generates a random LoadLiteral instruction
    pub fn gen_load_literal(&mut self) -> ValueId {
        // For now, only generate integer literals
        let literal_value: i32 = self.rng.random_range(-1000..1000);
        self.builder.add_load_literal(
            LiteralValue::Number(literal_value as f64), JsObjectType::Number
        )
    }

    /// Generates a random operation instruction
    pub fn gen_binary_operation(&mut self) -> ValueId {
        // pick two random values
        let values = self.builder.get_values_of_type(JsObjectType::Number);
        if values.len() < 2 {
            self.gen_load_literal();
            unimplemented!("Not enough values to generate binary operation");
        }
        let lhs = *values.choose(&mut self.rng).unwrap();
        let rhs = *values.choose(&mut self.rng).unwrap();

        let op = BinaryOperator::get_random_operator(&mut self.rng);
        self.builder.add_binary_op(op, lhs, rhs, JsObjectType::Number)
    }

    /// Generates a random object ctor
    pub fn gen_object_ctor(&mut self) -> ValueId {
        let obj_type = get_random_global_object(&mut self.rng);
        let ctor_signature = JsGlobalObject::get_constructor_signatures(
            &obj_type
        );
        let ctor_signature = ctor_signature.choose(&mut self.rng).unwrap();
        todo!();
    }
}

pub struct FunctionGenerator {
    pub rng: rand::rngs::ThreadRng,
    func_builder: FunctionILBuilder,
}

impl FunctionGenerator {
    fn generate_block(&mut self, budget: usize) -> Result<()> {
        // for now, generate only 1 block
        let mut block_gen = BlockGenerator::new(
            self.rng.clone(), &mut self.func_builder
        );
        block_gen.random_block(budget)?;
        Ok(())
    }

    pub fn generate_function(mut self, name: Option<String>) -> Result<FunctionIL> {
        self.func_builder = FunctionILBuilder::new(name);

        // generate blocks
        self.generate_block(7)?;
        Ok(self.func_builder.finish())
    }
}



/// Generates a random jellIL program.
impl CodeGenerator {
    pub fn new() -> Self {
        Self {
            rng: rand::thread_rng(),
            il: JellIL::new(),
        }
    }

    pub fn generate_program(&mut self, num_functions: usize) -> Result<&JellIL> {
        let func_name = format!("func_0");
        // for now, only generate one function: the top-level function
        let func_gen = FunctionGenerator {
            rng: self.rng.clone(),
            func_builder: FunctionILBuilder::new(Some(func_name.clone())),
        };
        let func_il = func_gen.generate_function(Some(func_name))?;
        self.il.set_top_level(func_il);
        Ok(&self.il)
    }
}

#[cfg(test)]
mod tests {
    use crate::parsing::parser::generate_js;

    use super::*;

    #[test]
    fn test_code_generator() {
        let mut code_gen = CodeGenerator::new();
        let il = code_gen.generate_program(1)
            .expect("Failed to generate program");
        println!("Generated IL: {:#?}", il);
    }
}
