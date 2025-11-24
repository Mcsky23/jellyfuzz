
// In order to make code generation and mutations easier, we propose an intermediate language (IL) 
// that is based on static single assignment (SSA) form, situating itself beneath the AST.
//
// Each instruction in jellIL produces a new value identified by a unique ID, which can be 
// referenced by subsequent instructions.
//
// jellIL contains the following instructions:
// - LoadLiteral(value)
// - LoadVar("var_name")
// - StoreVar("var_name", valueId)
//
// - BinaryOp(op, leftId, rightId)
// - UnaryOp(op, operandId)
// 
// - LoadProp(valueId, "property_name")
// - StoreProp(valueId, "property_name", valueId)
// - LoadElem(valueId, indexId)
// - StoreElem(valueId, indexId, valueId)
//
// - LoadFunc("function_name")
// - CallFunc(funcId, [argId1, argId2, ...])
// - CallMethod(objectId, "method_name", [argId1, argId2, ...])
//
// - NewObject(JsGlobalObject, [argId1, argId2, ...])
//
// - IfElse(conditionId, [then_instructions], [else_instructions])
// - 
// - TODO: Control flow, loops, functions

use rand::seq::IndexedRandom;
use serde::de;

use crate::mutators::js_objects::{js_objects::JsGlobalObject, js_types::JsObjectType};

/// Represents a single instruction.
pub struct Instr {
    pub id: ValueId,
    pub kind: InstrKind,
}

/// Represents a unique identifier for a value produced by an instruction.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct ValueId(pub usize);
/// Represents a unique identifier for a basic block.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlockId(pub usize);
#[derive(Clone, Debug)]
/// Information about a value, including its type.
struct ValueInfo { type_info: JsObjectType }

impl std::fmt::Debug for ValueId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "v{}", self.0)
    }
}

impl std::fmt::Debug for BlockId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "b{}", self.0)
    }
}

/// A basic block is defined by it's id, a list of arguments(values that are used inside it which 
/// are defined outside it), a list of instructions, and a terminator instruction that defines
/// how control flows out of the block.
pub struct BasicBlock {
    pub id: BlockId,
    pub args: Vec<ValueId>, // the arguments passed to this block(equivalent to phi nodes)
    pub instrs: Vec<Instr>,
    pub terminator: BlockTerminator,
}

/// Defines how control flows out of a basic block.
pub enum BlockTerminator {
    // unconditional jump to target block with arguments
    Goto { target: BlockId, args: Vec<ValueId> }, 
    // conditional branch based on condition value
    IfElse {
        condition: ValueId,
        then_block: BlockId,
        then_args: Vec<ValueId>,
        else_block: BlockId,
        else_args: Vec<ValueId>,
    },
    // return from function with optional return value
    Return(Option<ValueId>),
}

/// Represents a function in jellIL. The function arguments are the arguments of the entry block.
pub struct FunctionIL {
    name: Option<String>,
    entry: BlockId,
    blocks: Vec<BasicBlock>,
    values: Vec<ValueInfo>,
}

/// Represents a jellIL representation of a program.
pub struct JellIL {
    functions: Vec<FunctionIL>,
    top_level: FunctionIL,
}

#[derive(Clone, Debug)]
pub enum LiteralValue {
    Number(f64),
    String(String),
    Boolean(bool),
    Array(Vec<LiteralValue>),
    Null,
    Undefined,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum BinaryOperator {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Exp,

    BitOr,
    BitAnd,
    BitXor,
    LShift,
    RShift,
    ZeroFillRShift,
}

impl BinaryOperator {
    pub fn get_random_operator(rng: &mut rand::rngs::ThreadRng) -> Self {
        use BinaryOperator::*;
        let operators = vec![
            Add, Sub, Mul, Div, Mod, Exp,
            BitOr, BitAnd, BitXor, LShift, RShift, ZeroFillRShift,
        ];
        *operators.choose(rng).unwrap()
    }
}

impl std::fmt::Debug for BinaryOperator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BinaryOperator::Add => write!(f, "+"),
            BinaryOperator::Sub => write!(f, "-"),
            BinaryOperator::Mul => write!(f, "*"),
            BinaryOperator::Div => write!(f, "/"),
            BinaryOperator::Mod => write!(f, "%"),
            BinaryOperator::Exp => write!(f, "**"),
            BinaryOperator::BitOr => write!(f, "|"),
            BinaryOperator::BitAnd => write!(f, "&"),
            BinaryOperator::BitXor => write!(f, "^"),
            BinaryOperator::LShift => write!(f, "<<"),
            BinaryOperator::RShift => write!(f, ">>"),
            BinaryOperator::ZeroFillRShift => write!(f, ">>>"),
        }
    }
}

pub enum InstrKind {
    LoadLiteral(LiteralValue),
    LoadVar(String),
    StoreVar(String, ValueId),
    BinaryOp{ op: BinaryOperator, left: ValueId, right: ValueId },
    // UnaryOp(String, ValueId), TODO

    LoadProp { obj: ValueId, prop: String },
    StoreProp { obj: ValueId, prop: String, value: ValueId },
    LoadElem { obj: ValueId, index: ValueId },
    StoreElem { obj: ValueId, index: ValueId, value: ValueId },

    LoadFunc(String),
    CallFunc { func: ValueId, args: Vec<ValueId> },
    CallMethod { obj: ValueId, method: String, args: Vec<ValueId> },

    NewObject(JsGlobalObject, Vec<ValueId>),

    // condition, then branch, else branch
    IfElse { condition: ValueId, then_branch: Vec<Instr>, else_branch: Vec<Instr> }, 
}

impl std::fmt::Debug for InstrKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InstrKind::LoadLiteral(lit) => write!(f, "LoadLiteral({:?})", lit),
            InstrKind::LoadVar(name) => write!(f, "LoadVar({})", name),
            InstrKind::StoreVar(name, value) => write!(f, "StoreVar({}, {:?})", name, value),
            InstrKind::BinaryOp { op, left, right } => write!(f, "BinaryOp({:?}, {:?}, {:?})", op, left, right),
            InstrKind::LoadProp { obj, prop } => write!(f, "LoadProp({:?}, {})", obj, prop),
            InstrKind::StoreProp { obj, prop, value } => write!(f, "StoreProp({:?}, {}, {:?})", obj, prop, value),
            InstrKind::LoadElem { obj, index } => write!(f, "LoadElem({:?}, {:?})", obj, index),
            InstrKind::StoreElem { obj, index, value } => write!(f, "StoreElem({:?}, {:?}, {:?})", obj, index, value),
            InstrKind::LoadFunc(name) => write!(f, "LoadFunc({})", name),
            InstrKind::CallFunc { func, args } => write!(f, "CallFunc({:?}, {:?})", func, args),
            InstrKind::CallMethod { obj, method, args } => write!(f, "CallMethod({:?}, {}, {:?})", obj, method, args),
            InstrKind::NewObject(obj_type, args) => write!(f, "NewObject({:?}, {:?})", obj_type, args),
            InstrKind::IfElse { condition, then_branch: _, else_branch: _ } => write!(f, "IfElse({:?}, <then_branch>, <else_branch>)", condition),
        }
    }
}

impl std::fmt::Debug for Instr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?} = {:?}", self.id, self.kind)
    }
}

impl std::fmt::Debug for BasicBlock {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Block {:?} Args: {:?}", self.id, self.args)?;
        for instr in &self.instrs {
            writeln!(f, "  {:?}", instr)?;
        }
        writeln!(f, "  Terminator: {:?}", self.terminator)
    }
}

impl std::fmt::Debug for BlockTerminator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BlockTerminator::Goto { target, args } => write!(f, "Goto {:?} Args: {:?}", target, args),
            BlockTerminator::IfElse { condition, then_block, then_args, else_block, else_args } => {
                write!(f, "IfElse Cond: {:?} Then: {:?} Args: {:?} Else: {:?} Args: {:?}", 
                    condition, then_block, then_args, else_block, else_args)
            },
            BlockTerminator::Return(ret) => write!(f, "Return {:?}", ret),
        }
    }
}


impl std::fmt::Debug for FunctionIL {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Function {:?} Entry: {:?}", self.name, self.entry)?;
        for block in &self.blocks {
            writeln!(f, "{:?}", block)?;
        }
        Ok(())
    }
}

impl std::fmt::Debug for JellIL {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Top Level Function:")?;
        writeln!(f, "{:?}", self.top_level)?;
        for func in &self.functions {
            writeln!(f, "Function:")?;
            writeln!(f, "{:?}", func)?;
        }
        Ok(())
    }
}

pub struct BlockBuilder<'a> {
    block: BasicBlock,
    builder: &'a mut FunctionILBuilder,
}

pub struct FunctionILBuilder {
    next_value: usize,
    next_block: usize,
    func: FunctionIL,
}

impl<'a> BlockBuilder<'a> {
    pub fn new(args: Vec<ValueId>, builder: &'a mut FunctionILBuilder) -> BlockBuilder<'a> {
        let block_id = BlockId(builder.next_block);
        builder.next_block += 1;
        Self {
            block: BasicBlock {
                id: block_id,
                args,
                instrs: Vec::new(),
                terminator: BlockTerminator::Return(None), // placeholder
            },
            builder,
        }
    }

    pub fn add_instr(&mut self, kind: InstrKind, ty: JsObjectType) -> ValueId {
        let v = self.builder.new_value(ty);
        let instr = Instr { id: v, kind };
        self.block.instrs.push(instr);
        v
    }

    pub fn set_terminator(&mut self, term: BlockTerminator) {
        self.block.terminator = term;
    }

    pub fn finish(self) -> BlockId {
        let block_id = self.block.id;
        self.builder.func.blocks.push(self.block);
        block_id
    }

    pub fn get_values_of_type(&self, type_info: JsObjectType) -> Vec<ValueId> {
        self.builder.get_values_of_type(type_info)
    }

    pub fn get_all_values(&self) -> Vec<ValueId> {
        self.builder.func.values.iter().enumerate()
            .map(|(idx, _)| ValueId(idx))
            .collect()
    }

    // =========================== Code Generation Helpers ===========================

    pub fn add_binary_op(&mut self,
        op: BinaryOperator,
        left: ValueId,
        right: ValueId,
        ty: JsObjectType,
    ) -> ValueId {
        self.add_instr(
            InstrKind::BinaryOp { op, left, right },
            ty,
        )
    }

    // TODO: infer type based LiteralValue
    pub fn add_load_literal(&mut self, value: LiteralValue, ty: JsObjectType) -> ValueId {
        self.add_instr(
            InstrKind::LoadLiteral(value),
            ty,
        )
    }

    pub fn add_load_var(&mut self, name: String, ty: JsObjectType) -> ValueId {
        self.add_instr(
            InstrKind::LoadVar(name),
            ty,
        )
    }

    pub fn add_store_var(&mut self, name: String, value: ValueId) -> ValueId {
        self.add_instr(
            InstrKind::StoreVar(name, value),
            JsObjectType::Undefined,
        )
    }

    pub fn add_load_prop(&mut self, obj: ValueId, prop: String, ty: JsObjectType) -> ValueId {
        self.add_instr(
            InstrKind::LoadProp { obj, prop },
            ty,
        )
    }

    pub fn add_store_prop(&mut self, obj: ValueId, prop: String, value: ValueId) -> ValueId {
        self.add_instr(
            InstrKind::StoreProp { obj, prop, value },
            JsObjectType::Undefined,
        )
    }

    pub fn add_load_elem(
        &mut self, 
        obj: ValueId, 
        index: ValueId, 
        ty: JsObjectType
    ) -> ValueId {
        self.add_instr(
            InstrKind::LoadElem { obj, index },
            ty,
        )
    }

    pub fn add_store_elem(&mut self, obj: ValueId, index: ValueId, value: ValueId) -> ValueId {
        self.add_instr(
            InstrKind::StoreElem { obj, index, value },
            JsObjectType::Undefined,
        )
    }

    pub fn add_load_func(&mut self, name: String) -> ValueId {
        self.add_instr(
            InstrKind::LoadFunc(name),
            JsObjectType::Function,
        )
    }

    pub fn add_call_func(
        &mut self, func: ValueId, args: Vec<ValueId>, ty: JsObjectType
    ) -> ValueId {
        self.add_instr(
            InstrKind::CallFunc { func, args },
            ty,
        )
    }

    pub fn add_call_method(
        &mut self, obj: ValueId, method: String, args: Vec<ValueId>, ty: JsObjectType
    ) -> ValueId {
        self.add_instr(
            InstrKind::CallMethod { obj, method, args },
            ty,
        )
    }

    pub fn add_new_object(
        &mut self, obj_type: JsGlobalObject, args: Vec<ValueId>
    ) -> ValueId {
        let ty = obj_type.to_js_type();
        self.add_instr(
            InstrKind::NewObject(obj_type, args),
            ty,
        )
    }
}

impl FunctionILBuilder {
    pub fn new(name: Option<String>) -> Self {
        Self {
            next_value: 0,
            next_block: 0,
            func: FunctionIL {
                name,
                entry: BlockId(0),
                blocks: Vec::new(),
                values: Vec::new(),
            },
        }
    }

    pub fn new_block_builder(&mut self, args: Vec<ValueId>) -> BlockBuilder {
        BlockBuilder::new(args, self)
    }

    pub fn new_value(&mut self, type_info: JsObjectType) -> ValueId {
        let value_id = ValueId(self.next_value);
        self.next_value += 1;
        self.func.values.push(ValueInfo { type_info });
        value_id
    }

    pub fn get_values_of_type(&self, type_info: JsObjectType) -> Vec<ValueId> {
        self.func.values.iter().enumerate()
            .filter_map(|(idx, vinfo)| {
                if vinfo.type_info == type_info {
                    Some(ValueId(idx))
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn add_block_param(&mut self, block_id: BlockId, type_info: JsObjectType) -> ValueId {
        let v = self.new_value(type_info);
        if let Some(block) = self.func.blocks.iter_mut().find(
            |b| b.id == block_id
        ) {
            block.args.push(v);
        } else {
            panic!("Block ID not found");
        }
        v
    }

    pub fn add_instr(&mut self, block: BlockId, kind: InstrKind, ty: JsObjectType) -> ValueId {
        let v = self.new_value(ty);
        let instr = Instr { id: v, kind };
        let blk = self
            .func
            .blocks
            .iter_mut()
            .find(|b| b.id == block)
            .expect("block not found");
        blk.instrs.push(instr);
        v
    }

    pub fn set_term(&mut self, block: BlockId, term: BlockTerminator) {
        let blk = self
            .func
            .blocks
            .iter_mut()
            .find(|b| b.id == block)
            .expect("block not found");
        blk.terminator = term;
    }

    pub fn finish(self) -> FunctionIL {
        self.func
    }
}

impl JellIL {
    pub fn new() -> Self {
        Self {
            functions: Vec::new(),
            top_level: FunctionIL {
                name: None,
                entry: BlockId(0),
                blocks: Vec::new(),
                values: Vec::new(),
            },
        }
    }

    pub fn add_function(&mut self, func: FunctionIL) {
        self.functions.push(func);
    }

    pub fn set_top_level(&mut self, func: FunctionIL) {
        self.top_level = func;
    }
}

pub fn generate_random_il() -> JellIL {
    let mut il = JellIL::new();
    // for now just generate a default program
    let mut func = FunctionILBuilder::new(Some("main".to_string()));
    let mut block_builder = func.new_block_builder(Vec::new());
    let lhs = block_builder.add_load_literal(
        LiteralValue::Number(42.0),
        JsObjectType::Number,
    );
    let rhs = block_builder.add_load_literal(
        LiteralValue::Number(58.0),
        JsObjectType::Number,
    );
    let res = block_builder
        .add_binary_op(BinaryOperator::Add, lhs, rhs, JsObjectType::Number);
    block_builder.set_terminator(BlockTerminator::Return(Some(res)));
    block_builder.finish();
    let function_il = func.finish();
    il.top_level = function_il;
    il
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_random_il() {
        let il = generate_random_il();
        println!("{:#?}", il.top_level);
    }
}