
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

enum LiteralValue {
    Number(f64),
    String(String),
    Boolean(bool),
    Array(Vec<LiteralValue>),
    Null,
    Undefined,
}

enum BinaryOperator {
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

struct BlockBuilder<'a> {
    block: BasicBlock,
    block_id: BlockId,
    builder: &'a mut FunctionILBuilder,
}

struct FunctionILBuilder {
    next_value: usize,
    next_block: usize,
    func: FunctionIL,
}

impl BlockBuilder<'_> {
    pub fn add_instr(&mut self, kind: InstrKind, ty: JsObjectType) -> ValueId {
        let v = self.builder.new_value(ty);
        let instr = Instr { id: v, kind };
        self.block.instrs.push(instr);
        v
    }

    pub fn set_terminator(&mut self, term: BlockTerminator) {
        self.block.terminator = term;
    }

    pub fn finish(self) {
        self.builder.func.blocks.push(self.block);
    }

    pub fn id(&self) -> BlockId {
        self.block_id
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

    pub fn new_block(&mut self, args: Vec<ValueId>) -> BlockId {
        let block_id = BlockId(self.next_block);
        self.next_block += 1;
        self.func.blocks.push(BasicBlock {
            id: block_id,
            args,
            instrs: Vec::new(),
            terminator: BlockTerminator::Return(None), // placeholder
        });
        block_id
    }

    pub fn new_value(&mut self, type_info: JsObjectType) -> ValueId {
        let value_id = ValueId(self.next_value);
        self.next_value += 1;
        self.func.values.push(ValueInfo { type_info });
        value_id
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
}

pub fn generate_random_il() -> JellIL {
    let mut il = JellIL::new();

    il
}
