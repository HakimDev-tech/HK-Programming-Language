//! Mandatory verifier for compiler-produced in-memory HK bytecode.
use super::bytecode::{BytecodeProgram, Constant, FunctionProto, Opcode, SourceLocation};

const SUPPORTED_MAJOR: u16 = 1;
const SUPPORTED_MINOR: u16 = 0;
const SUPPORTED_FEATURES: u64 = 0;
const MAX_CONSTANTS: usize = 1_000_000;
const MAX_FUNCTIONS: usize = 65_535;
const MAX_MODULES: usize = 1_024;
const MAX_CODE_INSTRUCTIONS: usize = 1_000_000;
const MAX_OPERAND_STACK: u32 = 65_536;
const MAX_CALL_ARGUMENTS: u16 = 255;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct VerifyError {
    pub(super) function: Option<usize>,
    pub(super) instruction: Option<usize>,
    pub(super) message: String,
}

impl VerifyError {
    fn program(message: impl Into<String>) -> Self {
        Self {
            function: None,
            instruction: None,
            message: message.into(),
        }
    }

    fn at(function: usize, instruction: Option<usize>, message: impl Into<String>) -> Self {
        Self {
            function: Some(function),
            instruction,
            message: message.into(),
        }
    }
}

impl std::fmt::Display for VerifyError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match (self.function, self.instruction) {
            (Some(function), Some(instruction)) => write!(
                formatter,
                "Bytecode verification failed in function {function} at instruction {instruction}: {}",
                self.message
            ),
            (Some(function), None) => write!(
                formatter,
                "Bytecode verification failed in function {function}: {}",
                self.message
            ),
            _ => write!(formatter, "Bytecode verification failed: {}", self.message),
        }
    }
}

impl std::error::Error for VerifyError {}

#[derive(Clone, Debug)]
pub(super) struct VerifiedProgram {
    program: BytecodeProgram,
    verified_max_stacks: Vec<u32>,
}

impl VerifiedProgram {
    pub(super) fn program(&self) -> &BytecodeProgram {
        &self.program
    }

    pub(super) fn max_stack(&self, function: usize) -> Option<u32> {
        self.verified_max_stacks.get(function).copied()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct FlowState {
    stack_height: u32,
    initialized_locals: Vec<bool>,
}

pub(super) fn verify(program: BytecodeProgram) -> Result<VerifiedProgram, VerifyError> {
    verify_header(&program)?;
    verify_program_tables(&program)?;

    let mut verified_max_stacks = Vec::with_capacity(program.functions.len());
    for (function_index, function) in program.functions.iter().enumerate() {
        verify_function_tables(&program, function_index, function)?;
        let maximum = verify_control_flow(&program, function_index, function)?;
        if maximum != function.max_stack {
            return Err(VerifyError::at(
                function_index,
                None,
                format!(
                    "declared maximum stack height is {}, verifier computed {maximum}",
                    function.max_stack
                ),
            ));
        }
        verified_max_stacks.push(maximum);
    }

    Ok(VerifiedProgram {
        program,
        verified_max_stacks,
    })
}

fn verify_header(program: &BytecodeProgram) -> Result<(), VerifyError> {
    if program.header.major != SUPPORTED_MAJOR {
        return Err(VerifyError::program(format!(
            "unsupported bytecode major version {}",
            program.header.major
        )));
    }
    if program.header.minor > SUPPORTED_MINOR {
        return Err(VerifyError::program(format!(
            "unsupported bytecode minor version {}",
            program.header.minor
        )));
    }
    let unsupported = program.header.required_features & !SUPPORTED_FEATURES;
    if unsupported != 0 {
        return Err(VerifyError::program(format!(
            "unsupported required feature bits 0x{unsupported:x}"
        )));
    }
    Ok(())
}

fn verify_program_tables(program: &BytecodeProgram) -> Result<(), VerifyError> {
    if program.constants.len() > MAX_CONSTANTS {
        return Err(VerifyError::program(
            "constant pool exceeds the configured limit",
        ));
    }
    if program.functions.is_empty() || program.functions.len() > MAX_FUNCTIONS {
        return Err(VerifyError::program("invalid function table size"));
    }
    if program.modules.len() > MAX_MODULES {
        return Err(VerifyError::program(
            "module table exceeds the configured limit",
        ));
    }
    if program.entry_function as usize >= program.functions.len() {
        return Err(VerifyError::program("entry function index is out of range"));
    }
    for constant in &program.constants {
        if matches!(constant, Constant::Number(number) if !number.is_finite()) {
            return Err(VerifyError::program(
                "constant pool contains a non-finite number",
            ));
        }
    }
    for function in &program.functions {
        if function
            .module_id
            .is_some_and(|id| id as usize >= program.modules.len())
        {
            return Err(VerifyError::program(
                "function references an unknown module",
            ));
        }
    }
    for (module_index, module) in program.modules.iter().enumerate() {
        if module.initializer as usize >= program.functions.len() {
            return Err(VerifyError::program(format!(
                "module {module_index} initializer index is out of range"
            )));
        }
        let initializer = &program.functions[module.initializer as usize];
        if initializer.module_id != Some(module_index as u32) {
            return Err(VerifyError::program(format!(
                "module {module_index} initializer belongs to a different module"
            )));
        }
        for import in &module.imports {
            if import.local_slot >= module.binding_count {
                return Err(VerifyError::program(format!(
                    "module {module_index} import local slot is out of range"
                )));
            }
            let target = program
                .modules
                .get(import.target_module as usize)
                .ok_or_else(|| {
                    VerifyError::program(format!(
                        "module {module_index} import references an unknown module"
                    ))
                })?;
            if import.target_slot >= target.binding_count {
                return Err(VerifyError::program(format!(
                    "module {module_index} import target slot is out of range"
                )));
            }
        }
        for export in &module.exports {
            if export.slot >= module.binding_count {
                return Err(VerifyError::program(format!(
                    "module {module_index} export slot is out of range"
                )));
            }
        }
    }
    Ok(())
}

fn verify_function_tables(
    program: &BytecodeProgram,
    function_index: usize,
    function: &FunctionProto,
) -> Result<(), VerifyError> {
    if function.parameter_count > MAX_CALL_ARGUMENTS {
        return Err(VerifyError::at(
            function_index,
            None,
            "function parameter count exceeds 255",
        ));
    }
    if function.parameter_count > function.local_count {
        return Err(VerifyError::at(
            function_index,
            None,
            "invalid local-slot layout",
        ));
    }
    if function.code.is_empty() || function.code.len() > MAX_CODE_INSTRUCTIONS {
        return Err(VerifyError::at(
            function_index,
            None,
            "invalid function instruction count",
        ));
    }
    for (instruction_index, instruction) in function.code.iter().enumerate() {
        if instruction.source.source_id as usize >= program.source_files.len() {
            return Err(VerifyError::at(
                function_index,
                Some(instruction_index),
                "source-map file index is out of range",
            ));
        }
        if instruction.source.line == 0 || instruction.source.column == 0 {
            return Err(VerifyError::at(
                function_index,
                Some(instruction_index),
                "source positions must be one-based",
            ));
        }
        match instruction.opcode {
            Opcode::Constant(index) => {
                if index as usize >= program.constants.len() {
                    return Err(VerifyError::at(
                        function_index,
                        Some(instruction_index),
                        "constant index is out of range",
                    ));
                }
            }
            Opcode::GetProperty(index) => {
                if !matches!(
                    program.constants.get(index as usize),
                    Some(Constant::String(_))
                ) {
                    return Err(VerifyError::at(
                        function_index,
                        Some(instruction_index),
                        "property name must reference a string constant",
                    ));
                }
            }
            Opcode::DefineLocal(slot) | Opcode::GetLocal(slot) | Opcode::SetLocal(slot) => {
                if slot >= function.local_count {
                    return Err(VerifyError::at(
                        function_index,
                        Some(instruction_index),
                        "local slot is out of range",
                    ));
                }
            }
            Opcode::DefineModule(slot) | Opcode::GetModule(slot) | Opcode::SetModule(slot) => {
                let Some(module_id) = function.module_id else {
                    return Err(VerifyError::at(
                        function_index,
                        Some(instruction_index),
                        "module binding opcode used by a source-only function",
                    ));
                };
                if slot >= program.modules[module_id as usize].binding_count {
                    return Err(VerifyError::at(
                        function_index,
                        Some(instruction_index),
                        "module binding slot is out of range",
                    ));
                }
            }
            Opcode::GetImport(slot) => {
                let Some(module_id) = function.module_id else {
                    return Err(VerifyError::at(
                        function_index,
                        Some(instruction_index),
                        "import opcode used by a source-only function",
                    ));
                };
                if slot as usize >= program.modules[module_id as usize].imports.len() {
                    return Err(VerifyError::at(
                        function_index,
                        Some(instruction_index),
                        "import slot is out of range",
                    ));
                }
            }
            Opcode::Call(arguments) if arguments > MAX_CALL_ARGUMENTS => {
                return Err(VerifyError::at(
                    function_index,
                    Some(instruction_index),
                    "call argument count exceeds 255",
                ));
            }
            Opcode::Jump(target) | Opcode::JumpIfFalse(target)
                if target as usize >= function.code.len() =>
            {
                return Err(VerifyError::at(
                    function_index,
                    Some(instruction_index),
                    "jump target is outside the current function",
                ));
            }
            _ => {}
        }
    }
    Ok(())
}

fn verify_control_flow(
    _program: &BytecodeProgram,
    function_index: usize,
    function: &FunctionProto,
) -> Result<u32, VerifyError> {
    let mut states = vec![None::<FlowState>; function.code.len()];
    let mut worklist = std::collections::VecDeque::new();
    let mut initial_locals = vec![false; function.local_count as usize];
    initial_locals[..function.parameter_count as usize].fill(true);
    states[0] = Some(FlowState {
        stack_height: 0,
        initialized_locals: initial_locals,
    });
    worklist.push_back(0usize);
    let mut maximum = 0u32;
    let mut saw_return = false;

    while let Some(index) = worklist.pop_front() {
        let state = states[index]
            .clone()
            .expect("queued states are initialized");
        let instruction = &function.code[index];
        let mut next = state;
        let mut successors = Vec::with_capacity(2);

        let underflow = |needed: u32| {
            VerifyError::at(
                function_index,
                Some(index),
                format!(
                    "operand stack underflow: instruction needs {needed}, found {}",
                    next.stack_height
                ),
            )
        };

        match instruction.opcode {
            Opcode::Constant(_) | Opcode::Null | Opcode::True | Opcode::False => {
                next.stack_height += 1;
            }
            Opcode::Pop => {
                if next.stack_height < 1 {
                    return Err(underflow(1));
                }
                next.stack_height -= 1;
            }
            Opcode::Dup => {
                if next.stack_height < 1 {
                    return Err(underflow(1));
                }
                next.stack_height += 1;
            }
            Opcode::DefineLocal(slot) => {
                if next.stack_height < 1 {
                    return Err(underflow(1));
                }
                // A declaration inside a loop may execute repeatedly. The compiler
                // assigns stable, non-overlapping slots to lexical bindings; re-entering
                // the same declaration reinitializes that slot.
                next.initialized_locals[slot as usize] = true;
                next.stack_height -= 1;
            }
            Opcode::GetLocal(slot) => {
                if !next.initialized_locals[slot as usize] {
                    return Err(VerifyError::at(
                        function_index,
                        Some(index),
                        "read from a local slot that is not definitely initialized",
                    ));
                }
                next.stack_height += 1;
            }
            Opcode::SetLocal(slot) => {
                if !next.initialized_locals[slot as usize] {
                    return Err(VerifyError::at(
                        function_index,
                        Some(index),
                        "assignment to a local slot before initialization",
                    ));
                }
                if next.stack_height < 1 {
                    return Err(underflow(1));
                }
            }
            Opcode::DefineModule(_) => {
                if next.stack_height < 1 {
                    return Err(underflow(1));
                }
                next.stack_height -= 1;
            }
            Opcode::GetModule(_) | Opcode::GetImport(_) => {
                next.stack_height += 1;
            }
            Opcode::SetModule(_) => {
                if next.stack_height < 1 {
                    return Err(underflow(1));
                }
            }
            Opcode::EnterScope | Opcode::ExitScope => {}
            Opcode::Negate | Opcode::Not | Opcode::GetProperty(_) => {
                if next.stack_height < 1 {
                    return Err(underflow(1));
                }
            }
            Opcode::Add
            | Opcode::Subtract
            | Opcode::Multiply
            | Opcode::Divide
            | Opcode::Modulo
            | Opcode::Equal
            | Opcode::NotEqual
            | Opcode::Greater
            | Opcode::GreaterEqual
            | Opcode::Less
            | Opcode::LessEqual
            | Opcode::GetIndex => {
                if next.stack_height < 2 {
                    return Err(underflow(2));
                }
                next.stack_height -= 1;
            }
            Opcode::SetIndex => {
                if next.stack_height < 3 {
                    return Err(underflow(3));
                }
                next.stack_height -= 2;
            }
            Opcode::BuildArray(count) => {
                if next.stack_height < count as u32 {
                    return Err(underflow(count as u32));
                }
                next.stack_height = next.stack_height - count as u32 + 1;
            }
            Opcode::Call(arguments) => {
                let needed = arguments as u32 + 1;
                if next.stack_height < needed {
                    return Err(underflow(needed));
                }
                next.stack_height -= arguments as u32;
            }
            Opcode::Jump(target) => {
                successors.push(target as usize);
            }
            Opcode::JumpIfFalse(target) => {
                if next.stack_height < 1 {
                    return Err(underflow(1));
                }
                next.stack_height -= 1;
                successors.push(target as usize);
                if index + 1 < function.code.len() {
                    successors.push(index + 1);
                }
            }
            Opcode::Return => {
                if next.stack_height != 1 {
                    return Err(VerifyError::at(
                        function_index,
                        Some(index),
                        format!(
                            "return requires exactly one result on the operand stack, found {}",
                            next.stack_height
                        ),
                    ));
                }
                saw_return = true;
            }
        }

        maximum = maximum.max(next.stack_height);
        if maximum > MAX_OPERAND_STACK {
            return Err(VerifyError::at(
                function_index,
                Some(index),
                "operand stack exceeds the configured limit",
            ));
        }

        if !matches!(instruction.opcode, Opcode::Return | Opcode::Jump(_)) {
            if index + 1 < function.code.len() {
                successors.push(index + 1);
            } else {
                return Err(VerifyError::at(
                    function_index,
                    Some(index),
                    "reachable control flow falls off the end of the function",
                ));
            }
        }

        for successor in successors {
            merge_state(function_index, successor, &next, &mut states, &mut worklist)?;
        }
    }

    if !saw_return {
        return Err(VerifyError::at(
            function_index,
            None,
            "function has no reachable return instruction",
        ));
    }
    Ok(maximum)
}

fn merge_state(
    function_index: usize,
    target: usize,
    incoming: &FlowState,
    states: &mut [Option<FlowState>],
    worklist: &mut std::collections::VecDeque<usize>,
) -> Result<(), VerifyError> {
    match &mut states[target] {
        None => {
            states[target] = Some(incoming.clone());
            worklist.push_back(target);
        }
        Some(existing) => {
            if existing.stack_height != incoming.stack_height {
                return Err(VerifyError::at(
                    function_index,
                    Some(target),
                    format!(
                        "inconsistent operand-stack height at control-flow merge: {} versus {}",
                        existing.stack_height, incoming.stack_height
                    ),
                ));
            }
            let mut changed = false;
            for (current, next) in existing
                .initialized_locals
                .iter_mut()
                .zip(&incoming.initialized_locals)
            {
                let definitely_initialized = *current && *next;
                if *current != definitely_initialized {
                    *current = definitely_initialized;
                    changed = true;
                }
            }
            if changed {
                worklist.push_back(target);
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bytecode::{BytecodeHeader, FunctionProto, Instruction, SourceFile};

    fn instruction(opcode: Opcode) -> Instruction {
        Instruction {
            opcode,
            source: SourceLocation {
                source_id: 0,
                line: 1,
                column: 1,
            },
        }
    }

    fn function(code: Vec<Opcode>, max_stack: u32) -> BytecodeProgram {
        BytecodeProgram {
            header: BytecodeHeader::default(),
            constants: vec![Constant::Number(42.0)],
            source_files: vec![SourceFile {
                name: "test.hk".to_owned(),
            }],
            functions: vec![FunctionProto {
                name: "main".to_owned(),
                module_id: None,
                parameter_count: 0,
                local_count: 0,
                max_stack,
                code: code.into_iter().map(instruction).collect(),
            }],
            modules: Vec::new(),
            entry_function: 0,
        }
    }

    #[test]
    fn accepts_a_well_formed_constant_return_function() {
        let program = function(vec![Opcode::Constant(0), Opcode::Return], 1);
        let verified = verify(program).unwrap();
        assert_eq!(verified.max_stack(0), Some(1));
    }

    #[test]
    fn rejects_operand_stack_underflow() {
        let error =
            verify(function(vec![Opcode::Pop, Opcode::Null, Opcode::Return], 1)).unwrap_err();
        assert!(error.message.contains("underflow"));
    }

    #[test]
    fn rejects_out_of_range_jump_targets() {
        let error = verify(function(
            vec![Opcode::Jump(9), Opcode::Null, Opcode::Return],
            1,
        ))
        .unwrap_err();
        assert!(error.message.contains("jump target"));
    }

    #[test]
    fn rejects_inconsistent_stack_heights_at_control_flow_merges() {
        let program = function(
            vec![
                Opcode::True,
                Opcode::JumpIfFalse(5),
                Opcode::Null,
                Opcode::Null,
                Opcode::Jump(6),
                Opcode::Null,
                Opcode::Return,
            ],
            2,
        );
        let error = verify(program).unwrap_err();
        assert!(
            error.message.contains("inconsistent operand-stack height"),
            "unexpected verifier error: {}",
            error
        );
    }

    #[test]
    fn rejects_reads_of_locals_not_initialized_on_every_path() {
        let program = function(
            vec![
                Opcode::True,
                Opcode::JumpIfFalse(4),
                Opcode::Null,
                Opcode::DefineLocal(0),
                Opcode::GetLocal(0),
                Opcode::Pop,
                Opcode::Null,
                Opcode::Return,
            ],
            1,
        );
        let mut program = program;
        program.functions[0].local_count = 1;
        let error = verify(program).unwrap_err();
        assert!(error.message.contains("not definitely initialized"));
    }

    #[test]
    fn rejects_non_finite_constants() {
        let mut program = function(vec![Opcode::Null, Opcode::Return], 1);
        program.constants[0] = Constant::Number(f64::INFINITY);
        let error = verify(program).unwrap_err();
        assert!(error.message.contains("non-finite"));
    }
}
