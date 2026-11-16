#![allow(dead_code)] // Verifier entry points are integrated by the compiler/VM milestone.
//! Mandatory verifier for compiler-produced in-memory HK bytecode.
use super::bytecode::{BytecodeProgram, Constant, FunctionProto, Opcode};
use super::types::Type;

const SUPPORTED_MAJOR: u16 = 1;
const SUPPORTED_MINOR: u16 = 0;
const SUPPORTED_FEATURES: u64 = 0;
const SUPPORTED_COMPILER_ABI: u32 = 1;
const MAX_CONSTANTS: usize = 1_000_000;
const MAX_FUNCTIONS: usize = 65_535;
const MAX_MODULES: usize = 1_024;
const MAX_CODE_INSTRUCTIONS: usize = 1_000_000;
const MAX_OPERAND_STACK: u32 = 65_536;
// Aligns with the parser's existing maximum syntactic nesting depth.
const MAX_SCOPE_DEPTH: u32 = 128;
const MAX_CALL_ARGUMENTS: u16 = 255;
const MAX_OPERAND_SOURCES: usize = u16::MAX as usize;

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
    scope_depth: u32,
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
    if program.header.compiler_abi != SUPPORTED_COMPILER_ABI {
        return Err(VerifyError::program(format!(
            "unsupported compiler ABI version {}",
            program.header.compiler_abi
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
    let total_instructions = program
        .functions
        .iter()
        .try_fold(0usize, |total, function| {
            total.checked_add(function.code.len())
        });
    if total_instructions.is_none_or(|total| total > MAX_CODE_INSTRUCTIONS) {
        return Err(VerifyError::program(
            "aggregate bytecode instruction count exceeds the configured limit",
        ));
    }
    let total_bindings = program.modules.iter().try_fold(0usize, |total, module| {
        total.checked_add(module.bindings.len())
    });
    if total_bindings.is_none_or(|total| total > MAX_CONSTANTS) {
        return Err(VerifyError::program(
            "aggregate module binding count exceeds the configured limit",
        ));
    }
    if program.entry_function as usize >= program.functions.len() {
        return Err(VerifyError::program("entry function index is out of range"));
    }
    if program
        .source_files
        .iter()
        .any(|source| source.name.is_empty())
    {
        return Err(VerifyError::program("source-file names must not be empty"));
    }
    let entry = &program.functions[program.entry_function as usize];
    if entry.module_id.is_some() || !entry.parameters.is_empty() {
        return Err(VerifyError::program(
            "entry function must be source-only and accept no parameters",
        ));
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
    let mut module_names = std::collections::HashSet::new();
    for (module_index, module) in program.modules.iter().enumerate() {
        if module.name.is_empty() || !module_names.insert(module.name.as_str()) {
            return Err(VerifyError::program(format!(
                "module {module_index} has an empty or duplicate name"
            )));
        }
        if module.initializer as usize >= program.functions.len() {
            return Err(VerifyError::program(format!(
                "module {module_index} initializer index is out of range"
            )));
        }
        let initializer = &program.functions[module.initializer as usize];
        if !initializer.parameters.is_empty() {
            return Err(VerifyError::program(format!(
                "module {module_index} initializer must not accept parameters"
            )));
        }
        if initializer.module_id != Some(module_index as u32) {
            return Err(VerifyError::program(format!(
                "module {module_index} initializer belongs to a different module"
            )));
        }
        if module.bindings.len() > u16::MAX as usize + 1 {
            return Err(VerifyError::program(format!(
                "module {module_index} binding table exceeds the slot limit"
            )));
        }
        let mut internal_names = std::collections::HashSet::new();
        let mut source_names = std::collections::HashSet::new();
        for binding in &module.bindings {
            if binding.internal_name.is_empty()
                || binding.source_name.is_empty()
                || !internal_names.insert(binding.internal_name.as_str())
                || !source_names.insert(binding.source_name.as_str())
            {
                return Err(VerifyError::program(format!(
                    "module {module_index} has invalid binding metadata"
                )));
            }
        }
        let mut import_slots = std::collections::HashSet::new();
        for import in &module.imports {
            let Some(local_binding) = module.bindings.get(import.local_slot as usize) else {
                return Err(VerifyError::program(format!(
                    "module {module_index} import local slot is out of range"
                )));
            };
            if !import_slots.insert(import.local_slot) || local_binding.mutable {
                return Err(VerifyError::program(format!(
                    "module {module_index} import slot is duplicated or writable"
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
            if import.target_module as usize >= module_index {
                return Err(VerifyError::program(format!(
                    "module {module_index} dependencies must precede their importer"
                )));
            }
            let Some(target_binding) = target.bindings.get(import.target_slot as usize) else {
                return Err(VerifyError::program(format!(
                    "module {module_index} import target slot is out of range"
                )));
            };
            if import.imported_name.is_empty()
                || !target.exports.iter().any(|export| {
                    export.slot == import.target_slot && export.name == import.imported_name
                })
            {
                return Err(VerifyError::program(format!(
                    "module {module_index} import does not reference a declared target export"
                )));
            }
            if local_binding.contract != target_binding.contract {
                return Err(VerifyError::program(format!(
                    "module {module_index} import contract metadata does not match its target"
                )));
            }
        }
        let mut export_names = std::collections::HashSet::new();
        for export in &module.exports {
            if export.name.is_empty()
                || !export_names.insert(export.name.as_str())
                || export.slot as usize >= module.bindings.len()
                || module.bindings[export.slot as usize].source_name != export.name
                || import_slots.contains(&export.slot)
            {
                return Err(VerifyError::program(format!(
                    "module {module_index} has invalid or duplicate exports"
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
    if function.name.is_empty() {
        return Err(VerifyError::at(
            function_index,
            None,
            "function name must not be empty",
        ));
    }
    if function.parameters.len() > MAX_CALL_ARGUMENTS as usize {
        return Err(VerifyError::at(
            function_index,
            None,
            "function parameter count exceeds 255",
        ));
    }
    if function.local_slots.len() > u16::MAX as usize + 1
        || function.parameters.len() > function.local_slots.len()
    {
        return Err(VerifyError::at(
            function_index,
            None,
            "invalid local-slot layout",
        ));
    }
    let mut internal_names = std::collections::HashSet::new();
    for slot in &function.local_slots {
        if slot.internal_name.is_empty()
            || slot.source_name.is_empty()
            || !internal_names.insert(slot.internal_name.as_str())
        {
            return Err(VerifyError::at(
                function_index,
                None,
                "local slots require unique internal names and non-empty source names",
            ));
        }
    }
    let mut parameter_slots = std::collections::HashSet::new();
    for parameter in &function.parameters {
        let Some(slot) = function.local_slots.get(parameter.slot as usize) else {
            return Err(VerifyError::at(
                function_index,
                None,
                "parameter local slot is out of range",
            ));
        };
        if parameter.name.is_empty()
            || !slot.mutable
            || !parameter_slots.insert(parameter.slot)
            || slot.source_name != parameter.name
            || slot.contract != parameter.annotation.map(Type::from)
        {
            return Err(VerifyError::at(
                function_index,
                None,
                "parameter metadata does not match its local slot",
            ));
        }
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
        if instruction.operands.len() > MAX_OPERAND_SOURCES
            || instruction.operands.iter().any(|position| {
                position.source_id as usize >= program.source_files.len()
                    || position.line == 0
                    || position.column == 0
            })
        {
            return Err(VerifyError::at(
                function_index,
                Some(instruction_index),
                "operand source-map metadata is invalid",
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
            Opcode::MakeFunction(index) => {
                if index as usize >= program.functions.len() {
                    return Err(VerifyError::at(
                        function_index,
                        Some(instruction_index),
                        "function prototype index is out of range",
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
                if slot as usize >= function.local_slots.len() {
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
                if slot as usize >= program.modules[module_id as usize].bindings.len() {
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
    let mut initial_locals = vec![false; function.local_slots.len()];
    for parameter in &function.parameters {
        initial_locals[parameter.slot as usize] = true;
    }
    states[0] = Some(FlowState {
        stack_height: 0,
        scope_depth: 0,
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
                if next.stack_height < 1 {
                    return Err(underflow(1));
                }
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
            Opcode::GetModule(_)
            | Opcode::GetImport(_)
            | Opcode::LoadPrint
            | Opcode::MakeFunction(_) => {
                next.stack_height += 1;
            }
            Opcode::SetModule(_) => {
                if next.stack_height < 1 {
                    return Err(underflow(1));
                }
            }
            Opcode::EnterScope => {
                next.scope_depth += 1;
                if next.scope_depth > MAX_SCOPE_DEPTH {
                    return Err(VerifyError::at(
                        function_index,
                        Some(index),
                        "lexical scope depth exceeds the configured limit",
                    ));
                }
            }
            Opcode::ExitScope => {
                if next.scope_depth == 0 {
                    return Err(VerifyError::at(
                        function_index,
                        Some(index),
                        "lexical scope stack underflow",
                    ));
                }
                next.scope_depth -= 1;
            }
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
                if next.scope_depth != 0 {
                    return Err(VerifyError::at(
                        function_index,
                        Some(index),
                        "return leaves a lexical scope open",
                    ));
                }
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
            if existing.scope_depth != incoming.scope_depth {
                return Err(VerifyError::at(
                    function_index,
                    Some(target),
                    format!(
                        "inconsistent lexical-scope depth at control-flow merge: {} versus {}",
                        existing.scope_depth, incoming.scope_depth
                    ),
                ));
            }
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

#[cfg(feature = "fuzzing")]
pub(super) fn fuzz_verify(input: &[u8]) {
    use super::bytecode::{
        BytecodeHeader, ExportBinding, FunctionProto, ImportBinding, Instruction, LocalSlot,
        ModuleProto, SourceFile, SourceLocation,
    };

    fn take_byte(input: &[u8], cursor: &mut usize) -> u8 {
        let value = input.get(*cursor).copied().unwrap_or(0);
        *cursor = cursor.saturating_add(1);
        value
    }

    fn take_u16(input: &[u8], cursor: &mut usize) -> u16 {
        u16::from_le_bytes([take_byte(input, cursor), take_byte(input, cursor)])
    }

    fn take_u32(input: &[u8], cursor: &mut usize) -> u32 {
        u32::from_le_bytes([
            take_byte(input, cursor),
            take_byte(input, cursor),
            take_byte(input, cursor),
            take_byte(input, cursor),
        ])
    }

    fn take_u64(input: &[u8], cursor: &mut usize) -> u64 {
        u64::from_le_bytes([
            take_byte(input, cursor),
            take_byte(input, cursor),
            take_byte(input, cursor),
            take_byte(input, cursor),
            take_byte(input, cursor),
            take_byte(input, cursor),
            take_byte(input, cursor),
            take_byte(input, cursor),
        ])
    }

    let mut cursor = 0usize;
    let header = BytecodeHeader {
        major: if take_byte(input, &mut cursor) & 3 == 0 {
            0
        } else {
            1
        },
        minor: u16::from(take_byte(input, &mut cursor) % 2),
        required_features: u64::from(take_byte(input, &mut cursor) & 1),
        compiler_abi: u32::from(take_byte(input, &mut cursor) % 2),
    };

    let constant_count = (take_byte(input, &mut cursor) % 16) as usize;
    let mut constants = Vec::with_capacity(constant_count);
    for _ in 0..constant_count {
        if take_byte(input, &mut cursor) & 1 == 0 {
            constants.push(Constant::Number(f64::from_bits(take_u64(
                input,
                &mut cursor,
            ))));
        } else {
            let length = (take_byte(input, &mut cursor) % 16) as usize;
            let mut value = String::with_capacity(length);
            for _ in 0..length {
                value.push(char::from(take_byte(input, &mut cursor)));
            }
            constants.push(Constant::String(value));
        }
    }

    let parameter_count = (take_byte(input, &mut cursor) % 16) as u16;
    let local_count = parameter_count + (take_byte(input, &mut cursor) % 16) as u16;
    let parameters = (0..parameter_count)
        .map(|slot| super::bytecode::ParameterProto {
            name: format!("param{slot}"),
            slot,
            annotation: None,
        })
        .collect();
    let local_slots = (0..local_count)
        .map(|slot| super::bytecode::LocalSlot {
            internal_name: format!("\\0fuzz:{slot}"),
            source_name: if slot < parameter_count {
                format!("param{slot}")
            } else {
                format!("local{slot}")
            },
            mutable: true,
            contract: None,
        })
        .collect();
    let instruction_count = 1 + (take_byte(input, &mut cursor) % 64) as usize;
    let mut code = Vec::with_capacity(instruction_count);

    for _ in 0..instruction_count {
        let opcode = match take_byte(input, &mut cursor) % 38 {
            0 => Opcode::Constant(take_u32(input, &mut cursor)),
            1 => Opcode::Null,
            2 => Opcode::True,
            3 => Opcode::False,
            4 => Opcode::Pop,
            5 => Opcode::Dup,
            6 => Opcode::DefineLocal(take_u16(input, &mut cursor)),
            7 => Opcode::GetLocal(take_u16(input, &mut cursor)),
            8 => Opcode::SetLocal(take_u16(input, &mut cursor)),
            9 => Opcode::DefineModule(take_u16(input, &mut cursor)),
            10 => Opcode::GetModule(take_u16(input, &mut cursor)),
            11 => Opcode::SetModule(take_u16(input, &mut cursor)),
            12 => Opcode::GetImport(take_u16(input, &mut cursor)),
            13 => Opcode::EnterScope,
            14 => Opcode::ExitScope,
            15 => Opcode::Negate,
            16 => Opcode::Not,
            17 => Opcode::Add,
            18 => Opcode::Subtract,
            19 => Opcode::Multiply,
            20 => Opcode::Divide,
            21 => Opcode::Modulo,
            22 => Opcode::Equal,
            23 => Opcode::NotEqual,
            24 => Opcode::Greater,
            25 => Opcode::GreaterEqual,
            26 => Opcode::Less,
            27 => Opcode::LessEqual,
            28 => Opcode::Jump(take_u32(input, &mut cursor)),
            29 => Opcode::JumpIfFalse(take_u32(input, &mut cursor)),
            30 => Opcode::Call(take_u16(input, &mut cursor)),
            31 => Opcode::Return,
            32 => Opcode::BuildArray(take_u16(input, &mut cursor)),
            33 => Opcode::GetIndex,
            34 => Opcode::SetIndex,
            36 => Opcode::MakeFunction(take_u32(input, &mut cursor)),
            37 => Opcode::LoadPrint,
            _ => Opcode::GetProperty(take_u32(input, &mut cursor)),
        };
        let source = SourceLocation {
            source_id: u32::from(take_byte(input, &mut cursor) % 2),
            line: u32::from(take_byte(input, &mut cursor)),
            column: u32::from(take_byte(input, &mut cursor)),
        };
        let operand_count = (take_byte(input, &mut cursor) % 4) as usize;
        let operands = (0..operand_count)
            .map(|_| SourceLocation {
                source_id: u32::from(take_byte(input, &mut cursor) % 2),
                line: u32::from(take_byte(input, &mut cursor)),
                column: u32::from(take_byte(input, &mut cursor)),
            })
            .collect();
        code.push(Instruction {
            opcode,
            source,
            operands,
            typed_context: take_byte(input, &mut cursor) & 1 == 1,
        });
    }

    let entry = FunctionProto {
        name: "entry".to_owned(),
        module_id: None,
        parameters: Vec::new(),
        local_slots: Vec::new(),
        return_contract: None,
        typed: false,
        max_stack: u32::from(take_byte(input, &mut cursor) % 64),
        code,
    };
    let auxiliary = FunctionProto {
        name: "fuzz-function".to_owned(),
        module_id: None,
        parameters,
        local_slots,
        return_contract: None,
        typed: false,
        max_stack: 1,
        code: vec![
            Instruction {
                opcode: Opcode::Null,
                source: SourceLocation {
                    source_id: 0,
                    line: 1,
                    column: 1,
                },
                operands: Vec::new(),
                typed_context: false,
            },
            Instruction {
                opcode: Opcode::Return,
                source: SourceLocation {
                    source_id: 0,
                    line: 1,
                    column: 1,
                },
                operands: Vec::new(),
                typed_context: false,
            },
        ],
    };
    let mut functions = vec![entry, auxiliary];
    let module_count = (take_byte(input, &mut cursor) % 4) as usize;
    let mut modules = Vec::with_capacity(module_count);

    for module_index in 0..module_count {
        let binding_count = (take_byte(input, &mut cursor) % 4) as usize;
        let mut bindings = (0..binding_count)
            .map(|slot| LocalSlot {
                internal_name: format!("\\0fuzz:module{module_index}:slot{slot}"),
                source_name: format!("export{slot}"),
                mutable: true,
                contract: None,
            })
            .collect::<Vec<_>>();
        let import_count = if module_index == 0 {
            0
        } else {
            (take_byte(input, &mut cursor) % 4) as usize
        };
        let mut imports = Vec::with_capacity(import_count);
        for _ in 0..import_count {
            let local_slot = if binding_count == 0 {
                take_byte(input, &mut cursor) as u16
            } else {
                u16::from(take_byte(input, &mut cursor)) % binding_count as u16
            };
            let target_module = if module_count == 0 {
                0
            } else {
                u32::from(take_byte(input, &mut cursor)) % module_count as u32
            };
            let target_slot = u16::from(take_byte(input, &mut cursor) % 4);
            if let Some(binding) = bindings.get_mut(local_slot as usize) {
                binding.mutable = false;
            }
            imports.push(ImportBinding {
                imported_name: format!("export{target_slot}"),
                local_slot,
                target_module,
                target_slot,
            });
        }
        let exports = (0..binding_count)
            .filter(|slot| {
                !imports
                    .iter()
                    .any(|import| import.local_slot as usize == *slot)
            })
            .map(|slot| ExportBinding {
                name: format!("export{slot}"),
                slot: slot as u16,
            })
            .collect::<Vec<_>>();
        let initializer = functions.len() as u32;
        functions.push(FunctionProto {
            name: format!("fuzz-module-init-{module_index}"),
            module_id: Some(module_index as u32),
            parameters: Vec::new(),
            local_slots: Vec::new(),
            return_contract: None,
            typed: false,
            max_stack: 1,
            code: vec![
                Instruction {
                    opcode: Opcode::Null,
                    source: SourceLocation {
                        source_id: 0,
                        line: 1,
                        column: 1,
                    },
                    operands: Vec::new(),
                    typed_context: false,
                },
                Instruction {
                    opcode: Opcode::Return,
                    source: SourceLocation {
                        source_id: 0,
                        line: 1,
                        column: 1,
                    },
                    operands: Vec::new(),
                    typed_context: false,
                },
            ],
        });
        modules.push(ModuleProto {
            name: format!("fuzz-module-{module_index}.hk"),
            initializer,
            bindings,
            imports,
            exports,
        });
    }

    let program = BytecodeProgram {
        header,
        constants,
        source_files: vec![SourceFile {
            name: "<fuzz-bytecode>".to_owned(),
        }],
        functions,
        modules,
        entry_function: 0,
    };
    let _ = verify(program);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bytecode::{
        BytecodeHeader, ExportBinding, FunctionProto, ImportBinding, Instruction, LocalSlot,
        ModuleProto, SourceFile, SourceLocation,
    };

    fn instruction(opcode: Opcode) -> Instruction {
        Instruction {
            opcode,
            source: SourceLocation {
                source_id: 0,
                line: 1,
                column: 1,
            },
            operands: Vec::new(),
            typed_context: false,
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
                parameters: Vec::new(),
                local_slots: Vec::new(),
                return_contract: None,
                typed: false,
                max_stack,
                code: code.into_iter().map(instruction).collect(),
            }],
            modules: Vec::new(),
            entry_function: 0,
        }
    }
    fn local_slot(internal_name: &str, source_name: &str, mutable: bool) -> LocalSlot {
        LocalSlot {
            internal_name: internal_name.to_owned(),
            source_name: source_name.to_owned(),
            mutable,
            contract: None,
        }
    }

    fn module_function(name: &str, module_id: u32) -> FunctionProto {
        FunctionProto {
            name: name.to_owned(),
            module_id: Some(module_id),
            parameters: Vec::new(),
            local_slots: Vec::new(),
            return_contract: None,
            typed: false,
            max_stack: 1,
            code: vec![instruction(Opcode::Null), instruction(Opcode::Return)],
        }
    }

    fn module_program(reverse_dependency: bool) -> BytecodeProgram {
        let mut program = function(vec![Opcode::Null, Opcode::Return], 1);
        program.functions.push(module_function("dependency", 0));
        program.functions.push(module_function("root", 1));
        let dependency = ModuleProto {
            name: "dependency.hk".to_owned(),
            initializer: 1,
            bindings: vec![local_slot("\\0dependency:x", "x", !reverse_dependency)],
            imports: if reverse_dependency {
                vec![ImportBinding {
                    imported_name: "x".to_owned(),
                    local_slot: 0,
                    target_module: 1,
                    target_slot: 0,
                }]
            } else {
                Vec::new()
            },
            exports: if reverse_dependency {
                Vec::new()
            } else {
                vec![ExportBinding {
                    name: "x".to_owned(),
                    slot: 0,
                }]
            },
        };
        let root = ModuleProto {
            name: "root.hk".to_owned(),
            initializer: 2,
            bindings: vec![local_slot("\\0root:x", "x", reverse_dependency)],
            imports: if reverse_dependency {
                Vec::new()
            } else {
                vec![ImportBinding {
                    imported_name: "x".to_owned(),
                    local_slot: 0,
                    target_module: 0,
                    target_slot: 0,
                }]
            },
            exports: Vec::new(),
        };
        program.modules = vec![dependency, root];
        program
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
    fn rejects_out_of_range_function_prototype_references() {
        let error = verify(function(vec![Opcode::MakeFunction(1), Opcode::Return], 1)).unwrap_err();
        assert!(error
            .message
            .contains("function prototype index is out of range"));
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
        program.functions[0]
            .local_slots
            .push(super::super::bytecode::LocalSlot {
                internal_name: "\0slot0".to_owned(),
                source_name: "x".to_owned(),
                mutable: true,
                contract: None,
            });
        let error = verify(program).unwrap_err();
        assert!(error.message.contains("not definitely initialized"));
    }

    #[test]
    fn rejects_scope_depth_above_the_parser_limit() {
        let mut code = vec![Opcode::EnterScope; 129];
        code.extend(std::iter::repeat_n(Opcode::ExitScope, 129));
        code.extend([Opcode::Null, Opcode::Return]);
        let error = verify(function(code, 1)).unwrap_err();
        assert!(error.message.contains("lexical scope depth exceeds"));
    }

    #[test]
    fn rejects_scope_stack_underflow() {
        let error = verify(function(
            vec![Opcode::ExitScope, Opcode::Null, Opcode::Return],
            1,
        ))
        .unwrap_err();
        assert!(error.message.contains("lexical scope stack underflow"));
    }

    #[test]
    fn rejects_return_with_an_open_lexical_scope() {
        let error = verify(function(
            vec![Opcode::EnterScope, Opcode::Null, Opcode::Return],
            1,
        ))
        .unwrap_err();
        assert!(error.message.contains("return leaves a lexical scope open"));
    }

    #[test]
    fn rejects_inconsistent_scope_depth_at_control_flow_merges() {
        let program = function(
            vec![
                Opcode::True,
                Opcode::JumpIfFalse(5),
                Opcode::EnterScope,
                Opcode::Null,
                Opcode::Jump(6),
                Opcode::Null,
                Opcode::Return,
            ],
            1,
        );
        let error = verify(program).unwrap_err();
        assert!(
            error.message.contains("inconsistent lexical-scope depth"),
            "unexpected verifier error: {}",
            error
        );
    }
    #[test]
    fn accepts_dependency_first_module_binding_and_import_metadata() {
        verify(module_program(false)).unwrap();
    }

    #[test]
    fn rejects_duplicate_top_level_module_binding_names() {
        let mut program = module_program(false);
        program.modules[0]
            .bindings
            .push(local_slot("another-internal-id", "x", true));
        let error = verify(program).unwrap_err();
        assert!(error.message.contains("invalid binding metadata"));
    }

    #[test]
    fn rejects_re_exporting_an_imported_alias() {
        let mut program = module_program(false);
        program.modules[1].exports.push(ExportBinding {
            name: "x".to_owned(),
            slot: 0,
        });
        let error = verify(program).unwrap_err();
        assert!(error.message.contains("invalid or duplicate exports"));
    }

    #[test]
    fn rejects_imports_that_target_private_bindings() {
        let mut program = module_program(false);
        program.modules[1].imports[0].imported_name = "private".to_owned();
        let error = verify(program).unwrap_err();
        assert!(error
            .message
            .contains("does not reference a declared target export"));
    }

    #[test]
    fn rejects_modules_that_are_not_ordered_dependency_first() {
        let error = verify(module_program(true)).unwrap_err();
        assert!(error
            .message
            .contains("dependencies must precede their importer"));
    }

    #[test]
    fn rejects_local_assignment_stack_underflow() {
        let mut program = function(vec![Opcode::Null, Opcode::Return], 1);
        program.functions[0]
            .local_slots
            .push(local_slot("slot", "x", true));
        program.functions[0].code = vec![
            instruction(Opcode::Null),
            instruction(Opcode::DefineLocal(0)),
            instruction(Opcode::SetLocal(0)),
            instruction(Opcode::Null),
            instruction(Opcode::Return),
        ];
        let error = verify(program).unwrap_err();
        assert!(error.message.contains("underflow"));
    }

    #[test]
    fn rejects_module_assignment_stack_underflow() {
        let mut program = module_program(false);
        program.functions[1].code = vec![
            instruction(Opcode::SetModule(0)),
            instruction(Opcode::Null),
            instruction(Opcode::Return),
        ];
        let error = verify(program).unwrap_err();
        assert!(error.message.contains("underflow"));
    }

    #[test]
    fn accepts_local_assignment_to_immutable_slots_for_runtime_enforcement() {
        let mut program = function(
            vec![
                Opcode::Null,
                Opcode::DefineLocal(0),
                Opcode::Null,
                Opcode::SetLocal(0),
                Opcode::Return,
            ],
            1,
        );
        program.functions[0]
            .local_slots
            .push(local_slot("const-slot", "constant", false));
        verify(program).unwrap();
    }

    #[test]
    fn accepts_module_assignment_to_immutable_bindings_for_runtime_enforcement() {
        let mut program = module_program(false);
        program.modules[0].bindings[0].mutable = false;
        program.functions[1].code = vec![
            instruction(Opcode::Null),
            instruction(Opcode::SetModule(0)),
            instruction(Opcode::Return),
        ];
        verify(program).unwrap();
    }

    #[test]
    fn accepts_definition_and_reads_of_import_slots_for_runtime_enforcement() {
        let mut program = module_program(false);
        program.functions[2].code = vec![
            instruction(Opcode::Null),
            instruction(Opcode::DefineModule(0)),
            instruction(Opcode::GetModule(0)),
            instruction(Opcode::Pop),
            instruction(Opcode::Null),
            instruction(Opcode::Return),
        ];
        verify(program).unwrap();
    }

    #[test]
    fn accepts_assignment_to_imported_alias_slots_for_runtime_enforcement() {
        let mut program = module_program(false);
        program.functions[2].code = vec![
            instruction(Opcode::Null),
            instruction(Opcode::SetModule(0)),
            instruction(Opcode::Return),
        ];
        verify(program).unwrap();
    }

    #[test]
    fn rejects_invalid_primary_source_map_entries() {
        let mut program = function(vec![Opcode::Null, Opcode::Return], 1);
        program.functions[0].code[0].source.line = 0;
        let error = verify(program).unwrap_err();
        assert!(error.message.contains("source positions must be one-based"));
    }

    #[test]
    fn rejects_invalid_operand_source_map_entries() {
        let mut program = function(vec![Opcode::Null, Opcode::Return], 1);
        program.functions[0].code[0].operands.push(SourceLocation {
            source_id: 1,
            line: 1,
            column: 1,
        });
        let error = verify(program).unwrap_err();
        assert!(error.message.contains("operand source-map metadata is invalid"));
    }

    #[test]
    fn rejects_unsupported_compiler_abi_versions() {
        let mut program = function(vec![Opcode::Null, Opcode::Return], 1);
        program.header.compiler_abi = 99;
        let error = verify(program).unwrap_err();
        assert!(error.message.contains("unsupported compiler ABI version"));
    }

    #[test]
    fn rejects_non_finite_constants() {
        let mut program = function(vec![Opcode::Null, Opcode::Return], 1);
        program.constants[0] = Constant::Number(f64::INFINITY);
        let error = verify(program).unwrap_err();
        assert!(error.message.contains("non-finite"));
    }
}
