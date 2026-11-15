//! In-memory bytecode representation for HK 0.4.x.
//!
//! This is an internal format, not a serialized or stable public ABI.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct BytecodeHeader {
    pub(super) major: u16,
    pub(super) minor: u16,
    pub(super) required_features: u64,
}

impl Default for BytecodeHeader {
    fn default() -> Self {
        Self {
            major: 1,
            minor: 0,
            required_features: 0,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(super) enum Constant {
    Number(f64),
    String(String),
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct SourceLocation {
    pub(super) source_id: u32,
    pub(super) line: u32,
    pub(super) column: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Opcode {
    Constant(u32),
    Null,
    True,
    False,
    Pop,
    Dup,
    DefineLocal(u16),
    GetLocal(u16),
    SetLocal(u16),
    DefineModule(u16),
    GetModule(u16),
    SetModule(u16),
    GetImport(u16),
    EnterScope,
    ExitScope,
    Negate,
    Not,
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,
    Equal,
    NotEqual,
    Greater,
    GreaterEqual,
    Less,
    LessEqual,
    Jump(u32),
    JumpIfFalse(u32),
    Call(u16),
    Return,
    BuildArray(u16),
    GetIndex,
    SetIndex,
    GetProperty(u32),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Instruction {
    pub(super) opcode: Opcode,
    pub(super) source: SourceLocation,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct SourceFile {
    pub(super) name: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct FunctionProto {
    pub(super) name: String,
    pub(super) module_id: Option<u32>,
    pub(super) parameter_count: u16,
    pub(super) local_count: u16,
    /// Must equal the maximum reachable operand-stack height recomputed by the verifier.
    pub(super) max_stack: u32,
    pub(super) code: Vec<Instruction>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct ImportBinding {
    pub(super) local_slot: u16,
    pub(super) target_module: u32,
    pub(super) target_slot: u16,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct ExportBinding {
    pub(super) name: String,
    pub(super) slot: u16,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct ModuleProto {
    pub(super) name: String,
    pub(super) initializer: u32,
    pub(super) binding_count: u16,
    pub(super) imports: Vec<ImportBinding>,
    pub(super) exports: Vec<ExportBinding>,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct BytecodeProgram {
    pub(super) header: BytecodeHeader,
    pub(super) constants: Vec<Constant>,
    pub(super) source_files: Vec<SourceFile>,
    pub(super) functions: Vec<FunctionProto>,
    pub(super) modules: Vec<ModuleProto>,
    pub(super) entry_function: u32,
}

impl BytecodeProgram {
    pub(super) fn empty_function(name: impl Into<String>, code: Vec<Instruction>) -> Self {
        Self {
            header: BytecodeHeader::default(),
            constants: Vec::new(),
            source_files: vec![SourceFile {
                name: "<generated>".to_owned(),
            }],
            functions: vec![FunctionProto {
                name: name.into(),
                module_id: None,
                parameter_count: 0,
                local_count: 0,
                max_stack: 0,
                code,
            }],
            modules: Vec::new(),
            entry_function: 0,
        }
    }
}
