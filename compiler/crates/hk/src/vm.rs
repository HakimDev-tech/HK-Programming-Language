//! Verified bytecode execution engine. The tree-walking engine remains the reference.
#![allow(dead_code)] // Public execution routing is gated on differential conformance.
use super::ast::Param;
use super::bytecode::{BytecodeProgram, Constant, Instruction, Opcode, UpvalueSource};
use super::diagnostics::HkError;
use super::environment::{Env, EnvError};
use super::lexer::K;
use super::runtime_ops::{self, RuntimeOpError};
use super::types::Type;
use super::value::{contains_array_reference, ensure_contract, Fun, FunctionBody, MathFn, V};
use super::verifier::VerifiedProgram;
use super::Result;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Clone)]
struct Frame {
    function: u32,
    ip: usize,
    env: Rc<RefCell<Env>>,
    scopes: Vec<Rc<RefCell<Env>>>,
    closure_env: Option<Rc<RefCell<Env>>>,
    stack_base: usize,
    source: Rc<String>,
    source_text: Rc<String>,
    typed: bool,
}

struct Vm<'a> {
    program: &'a BytecodeProgram,
    stack: Vec<V>,
    typed_stack: Vec<bool>,
    frames: Vec<Frame>,
    output: Vec<String>,
    max_call_depth: usize,
}

impl<'a> Vm<'a> {
    fn new(program: &'a BytecodeProgram, source_name: &str, source_text: &str) -> Self {
        let env = Rc::new(RefCell::new(Env::default()));
        Env::ensure_print(&env);
        let source = Rc::new(source_name.to_owned());
        let source_text = Rc::new(source_text.to_owned());
        Self {
            program,
            stack: Vec::new(),
            typed_stack: Vec::new(),
            frames: vec![Frame {
                function: program.entry_function,
                ip: 0,
                env: env.clone(),
                scopes: vec![env],
                closure_env: None,
                stack_base: 0,
                source,
                source_text,
                typed: false,
            }],
            output: Vec::new(),
            max_call_depth: 64,
        }
    }

    fn new_empty(program: &'a BytecodeProgram) -> Self {
        Self {
            program,
            stack: Vec::new(),
            typed_stack: Vec::new(),
            frames: Vec::new(),
            output: Vec::new(),
            max_call_depth: 64,
        }
    }

    fn error(
        &self,
        frame: &Frame,
        code: &'static str,
        message: impl Into<String>,
        position: (usize, usize),
    ) -> HkError {
        HkError::new(
            code,
            message,
            &frame.source,
            position.0,
            position.1,
            &frame.source_text,
        )
    }

    fn type_error(
        &self,
        frame: &Frame,
        typed: bool,
        message: impl Into<String>,
        position: (usize, usize),
    ) -> HkError {
        self.error(
            frame,
            if typed { "E3008" } else { "E3000" },
            message,
            position,
        )
    }

    fn env_error(&self, frame: &Frame, error: EnvError, position: (usize, usize)) -> HkError {
        match error {
            EnvError::Contract(message) => self.error(frame, "E3008", message, position),
            EnvError::ImportedAlias(message) => self.error(frame, "E4007", message, position),
            EnvError::Duplicate(message)
            | EnvError::Constant(message)
            | EnvError::Undefined(message) => self.error(frame, "E3000", message, position),
        }
    }

    fn push(&mut self, value: V, typed: bool) {
        self.stack.push(value);
        self.typed_stack.push(typed);
    }

    fn pop(&mut self, frame: &Frame, instruction: &Instruction) -> Result<(V, bool)> {
        let value = self.stack.pop().ok_or_else(|| {
            self.error(
                frame,
                "E4009",
                "Verified bytecode violated its operand-stack contract.",
                position(instruction),
            )
        })?;
        let typed = self.typed_stack.pop().unwrap_or(false);
        Ok((value, typed))
    }

    fn constant_string(
        &self,
        frame: &Frame,
        index: u32,
        instruction: &Instruction,
    ) -> Result<String> {
        match self.program.constants.get(index as usize) {
            Some(Constant::String(value)) => Ok(value.clone()),
            _ => Err(self.error(
                frame,
                "E4009",
                "Verified name operand is not a string constant.",
                position(instruction),
            )),
        }
    }

    fn binding_is_typed(&self, env: &Rc<RefCell<Env>>, name: &str) -> bool {
        Env::has_contract(env, name)
            || matches!(Env::get(env, name), Some(V::Fun(function)) if function.typed)
    }

    fn local_name(&self, frame: &Frame, slot: u16, instruction: &Instruction) -> Result<String> {
        self.program.functions[frame.function as usize]
            .local_slots
            .get(slot as usize)
            .map(|local| local.source_name.clone())
            .ok_or_else(|| {
                self.error(
                    frame,
                    "E4009",
                    "Verified local slot is out of range.",
                    position(instruction),
                )
            })
    }

    fn capture_name(&self, function_index: u32, slot: u16) -> Option<String> {
        let function = self.program.functions.get(function_index as usize)?;
        let parent_index = function.enclosing_function?;
        let capture = function.upvalues.get(slot as usize)?;
        match capture.source {
            UpvalueSource::EnclosingLocal(parent_slot) => self
                .program
                .functions
                .get(parent_index as usize)?
                .local_slots
                .get(parent_slot as usize)
                .map(|local| local.source_name.clone()),
            UpvalueSource::EnclosingUpvalue(parent_slot) => {
                self.capture_name(parent_index, parent_slot)
            }
        }
    }

    fn position_of_operand(instruction: &Instruction, index: usize) -> (usize, usize) {
        instruction.operands.get(index).map_or_else(
            || position(instruction),
            |operand| (operand.line as usize, operand.column as usize),
        )
    }

    fn call(
        &mut self,
        frame: &Frame,
        instruction: &Instruction,
        argument_count: usize,
    ) -> Result<()> {
        let mut arguments = Vec::with_capacity(argument_count);
        for index in (0..argument_count).rev() {
            let (value, typed) = self.pop(frame, instruction)?;
            arguments.push((
                value,
                typed,
                Self::position_of_operand(instruction, index + 1),
            ));
        }
        arguments.reverse();
        let arguments_typed = arguments.iter().any(|(_, typed, _)| *typed);
        let (callee, _callee_typed) = self.pop(frame, instruction)?;
        let call_position = position(instruction);
        match callee {
            V::Print => {
                if arguments.len() != 1 {
                    return Err(self.error(
                        frame,
                        "E3000",
                        format!(
                            "print() expects one argument, received {}.",
                            arguments.len()
                        ),
                        call_position,
                    ));
                }
                self.output.push(arguments[0].0.show());
                self.push(V::Null, arguments_typed);
            }
            V::Math(function) => {
                let expected = match function {
                    MathFn::Min | MathFn::Max => 2,
                    _ => 1,
                };
                if arguments.len() != expected {
                    return Err(self.error(
                        frame,
                        "E3011",
                        format!(
                            "std:math function expects {expected} argument(s), received {}.",
                            arguments.len()
                        ),
                        call_position,
                    ));
                }
                let mut numbers = Vec::with_capacity(arguments.len());
                for (value, _argument_typed, argument_position) in &arguments {
                    match value {
                        V::Num(number) if number.is_finite() => numbers.push(*number),
                        V::Num(_) => {
                            return Err(self.error(
                                frame,
                                "E3012",
                                "std:math arguments must be finite Numbers.",
                                *argument_position,
                            ))
                        }
                        _ => {
                            return Err(self.error(
                                frame,
                                "E3012",
                                "std:math arguments must be Numbers.",
                                *argument_position,
                            ))
                        }
                    }
                }
                let result = match function {
                    MathFn::Abs => numbers[0].abs(),
                    MathFn::Ceil => numbers[0].ceil(),
                    MathFn::Floor => numbers[0].floor(),
                    MathFn::Round => numbers[0].round(),
                    MathFn::Sqrt if numbers[0] < 0.0 => {
                        return Err(self.error(
                            frame,
                            "E3013",
                            "std:math::sqrt requires a non-negative argument.",
                            call_position,
                        ));
                    }
                    MathFn::Sqrt => numbers[0].sqrt(),
                    MathFn::Min => numbers[0].min(numbers[1]),
                    MathFn::Max => numbers[0].max(numbers[1]),
                };
                if !result.is_finite() {
                    return Err(self.error(
                        frame,
                        "E3014",
                        "std:math produced a non-finite result.",
                        call_position,
                    ));
                }
                self.push(V::Num(result), arguments_typed);
            }
            V::Fun(function) => {
                if arguments.len() != function.params.len() {
                    return Err(self.error(
                        frame,
                        "E3000",
                        format!(
                            "Function '{}' expects {} arguments, received {}.",
                            function.name,
                            function.params.len(),
                            arguments.len()
                        ),
                        call_position,
                    ));
                }
                if self.frames.len().saturating_sub(1) >= self.max_call_depth {
                    return Err(self.error(
                        frame,
                        "E3000",
                        "Maximum function call depth (64) exceeded.",
                        call_position,
                    ));
                }
                let parent = function.env.upgrade().ok_or_else(|| {
                    self.error(
                        frame,
                        "E3000",
                        "Function environment is no longer available.",
                        call_position,
                    )
                })?;
                let call_env = Env::child(parent.clone());
                for (parameter, (value, _argument_typed, argument_position)) in
                    function.params.iter().zip(arguments)
                {
                    call_env
                        .borrow_mut()
                        .define(
                            parameter.name.clone(),
                            value,
                            true,
                            parameter.annotation.map(Type::from),
                            "function parameter",
                        )
                        .map_err(|error| self.env_error(frame, error, argument_position))?;
                }
                let FunctionBody::Bytecode(function_index) = function.body else {
                    return Err(self.error(
                        frame,
                        "E4009",
                        "The bytecode VM cannot execute a tree-walking function value.",
                        call_position,
                    ));
                };
                let prototype = self
                    .program
                    .functions
                    .get(function_index as usize)
                    .ok_or_else(|| {
                        self.error(
                            frame,
                            "E4009",
                            "Function value references a missing bytecode prototype.",
                            call_position,
                        )
                    })?;
                self.frames.push(Frame {
                    function: function_index,
                    ip: 0,
                    env: call_env.clone(),
                    scopes: vec![call_env],
                    closure_env: Some(parent),
                    stack_base: self.stack.len(),
                    source: function.source.clone(),
                    source_text: function.source_text.clone(),
                    typed: prototype.typed,
                });
            }
            value => {
                return Err(self.error(
                    frame,
                    "E3000",
                    format!("Cannot call a value of type {}.", value.ty()),
                    call_position,
                ));
            }
        }
        Ok(())
    }

    fn execute_instruction(&mut self, instruction: Instruction) -> Result<()> {
        let frame_index = self.frames.len() - 1;
        let frame_snapshot = self.frames[frame_index].clone();
        let frame = &frame_snapshot;
        let pos = position(&instruction);
        let typed = instruction.typed_context || frame.typed;
        match instruction.opcode {
            Opcode::Constant(index) => {
                let value = match self.program.constants.get(index as usize) {
                    Some(Constant::Number(number)) => V::Num(*number),
                    Some(Constant::String(string)) => V::Str(string.clone()),
                    None => {
                        return Err(self.error(
                            frame,
                            "E4009",
                            "Constant index out of range after verification.",
                            pos,
                        ))
                    }
                };
                self.push(value, false);
            }
            Opcode::Null => self.push(V::Null, false),
            Opcode::True => self.push(V::Bool(true), false),
            Opcode::False => self.push(V::Bool(false), false),
            Opcode::Pop => {
                self.pop(frame, &instruction)?;
            }
            Opcode::Dup => {
                let value = self.stack.last().cloned().ok_or_else(|| {
                    self.error(
                        frame,
                        "E4009",
                        "Verified DUP encountered an empty stack.",
                        pos,
                    )
                })?;
                self.push(value, *self.typed_stack.last().unwrap_or(&false));
            }
            Opcode::DefineLocal(slot) | Opcode::DefineFunctionLocal(slot) => {
                let (value, _value_typed) = self.pop(frame, &instruction)?;
                let name = self.local_name(frame, slot, &instruction)?;
                let result = if matches!(instruction.opcode, Opcode::DefineFunctionLocal(_)) {
                    frame.env.borrow_mut().define_function(name, value)
                } else {
                    let local =
                        &self.program.functions[frame.function as usize].local_slots[slot as usize];
                    frame.env.borrow_mut().define(
                        name,
                        value,
                        local.mutable,
                        local.contract,
                        "variable initialization",
                    )
                };
                result.map_err(|error| self.env_error(frame, error, pos))?;
            }
            Opcode::GetLocal(slot) => {
                let name = self.local_name(frame, slot, &instruction)?;
                let value = Env::get(&frame.env, &name).ok_or_else(|| {
                    self.error(frame, "E3000", format!("Undefined variable '{name}'."), pos)
                })?;
                let typed_value = self.binding_is_typed(&frame.env, &name);
                self.push(value, typed_value);
            }
            Opcode::SetLocal(slot) => {
                let name = self.local_name(frame, slot, &instruction)?;
                let value = self.stack.last().cloned().ok_or_else(|| {
                    self.error(
                        frame,
                        "E4009",
                        "Verified SET_LOCAL encountered an empty stack.",
                        pos,
                    )
                })?;
                let value_typed = *self.typed_stack.last().unwrap_or(&false);
                let binding_typed = Env::has_contract(&frame.env, &name);
                Env::set(&frame.env, &name, value)
                    .map_err(|error| self.env_error(frame, error, pos))?;
                if let Some(last) = self.typed_stack.last_mut() {
                    *last = value_typed || binding_typed;
                }
            }
            Opcode::GetName(index) => {
                let name = self.constant_string(frame, index, &instruction)?;
                let value = Env::get(&frame.env, &name).ok_or_else(|| {
                    self.error(frame, "E3000", format!("Undefined variable '{name}'."), pos)
                })?;
                let typed_value = self.binding_is_typed(&frame.env, &name);
                self.push(value, typed_value);
            }
            Opcode::SetName(index) => {
                let name = self.constant_string(frame, index, &instruction)?;
                let value = self.stack.last().cloned().ok_or_else(|| {
                    self.error(
                        frame,
                        "E4009",
                        "Verified SET_NAME encountered an empty stack.",
                        pos,
                    )
                })?;
                let value_typed = *self.typed_stack.last().unwrap_or(&false);
                let binding_typed = Env::has_contract(&frame.env, &name);
                Env::set(&frame.env, &name, value)
                    .map_err(|error| self.env_error(frame, error, pos))?;
                if let Some(last) = self.typed_stack.last_mut() {
                    *last = value_typed || binding_typed;
                }
            }
            Opcode::GetUpvalue(slot) => {
                let name = self.capture_name(frame.function, slot).ok_or_else(|| {
                    self.error(
                        frame,
                        "E4009",
                        "Verified upvalue descriptor could not be resolved.",
                        pos,
                    )
                })?;
                let closure_env = frame.closure_env.as_ref().ok_or_else(|| {
                    self.error(
                        frame,
                        "E3000",
                        "Function environment is no longer available.",
                        pos,
                    )
                })?;
                let value = Env::get(closure_env, &name).ok_or_else(|| {
                    self.error(frame, "E3000", format!("Undefined variable '{name}'."), pos)
                })?;
                let typed_value = self.binding_is_typed(closure_env, &name);
                self.push(value, typed_value);
            }
            Opcode::SetUpvalue(slot) => {
                let name = self.capture_name(frame.function, slot).ok_or_else(|| {
                    self.error(
                        frame,
                        "E4009",
                        "Verified upvalue descriptor could not be resolved.",
                        pos,
                    )
                })?;
                let closure_env = frame.closure_env.as_ref().ok_or_else(|| {
                    self.error(
                        frame,
                        "E3000",
                        "Function environment is no longer available.",
                        pos,
                    )
                })?;
                let value = self.stack.last().cloned().ok_or_else(|| {
                    self.error(
                        frame,
                        "E4009",
                        "Verified SET_UPVALUE encountered an empty stack.",
                        pos,
                    )
                })?;
                let value_typed = *self.typed_stack.last().unwrap_or(&false);
                let binding_typed = Env::has_contract(closure_env, &name);
                Env::set(closure_env, &name, value)
                    .map_err(|error| self.env_error(frame, error, pos))?;
                if let Some(last) = self.typed_stack.last_mut() {
                    *last = value_typed || binding_typed;
                }
            }
            Opcode::LoadPrint => self.push(V::Print, false),
            Opcode::MakeFunction(function_index) => {
                let prototype = &self.program.functions[function_index as usize];
                let params = prototype
                    .parameters
                    .iter()
                    .map(|parameter| Param {
                        name: parameter.name.clone(),
                        annotation: parameter.annotation,
                    })
                    .collect();
                let source = frame.source.clone();
                let source_text = frame.source_text.clone();
                self.push(
                    V::Fun(Rc::new(Fun {
                        name: prototype.name.clone(),
                        params,
                        return_type: prototype.return_contract,
                        typed: prototype.typed,
                        body: FunctionBody::Bytecode(function_index),
                        env: Rc::downgrade(&frame.env),
                        source,
                        source_text,
                        line: pos.0,
                        column: pos.1,
                    })),
                    false,
                );
            }
            Opcode::EnterScope => {
                let child = Env::child(frame.env.clone());
                let frame = &mut self.frames[frame_index];
                frame.env = child.clone();
                frame.scopes.push(child);
            }
            Opcode::ExitScope => {
                let frame = &mut self.frames[frame_index];
                frame.scopes.pop();
                frame.env = frame.scopes.last().cloned().ok_or_else(|| {
                    HkError::new(
                        "E4009",
                        "Verified scope stack underflow.",
                        &frame.source,
                        pos.0,
                        pos.1,
                        &frame.source_text,
                    )
                })?;
            }
            Opcode::Negate | Opcode::Not => {
                let (value, value_typed) = self.pop(frame, &instruction)?;
                let operator = if matches!(instruction.opcode, Opcode::Negate) {
                    K::Minus
                } else {
                    K::Bang
                };
                let operand_pos = Self::position_of_operand(&instruction, 0);
                let value = runtime_ops::unary(value, operator, operand_pos, pos)
                    .map_err(|error| self.operation_error(frame, typed || value_typed, error))?;
                self.push(value, typed || value_typed);
            }
            Opcode::AssertBoolean => match self.stack.last() {
                Some(V::Bool(_)) => {}
                Some(value) => {
                    return Err(self.type_error(
                        frame,
                        typed || *self.typed_stack.last().unwrap_or(&false),
                        format!("Condition must be Boolean, received {}.", value.ty()),
                        pos,
                    ))
                }
                None => {
                    return Err(self.error(
                        frame,
                        "E4009",
                        "Verified boolean assertion encountered an empty stack.",
                        pos,
                    ))
                }
            },
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
            | Opcode::LessEqual => {
                let (right, right_typed) = self.pop(frame, &instruction)?;
                let (left, left_typed) = self.pop(frame, &instruction)?;
                let operator = match instruction.opcode {
                    Opcode::Add => K::Plus,
                    Opcode::Subtract => K::Minus,
                    Opcode::Multiply => K::Star,
                    Opcode::Divide => K::Slash,
                    Opcode::Modulo => K::Percent,
                    Opcode::Equal => K::EqEq,
                    Opcode::NotEqual => K::BangEq,
                    Opcode::Greater => K::Gt,
                    Opcode::GreaterEqual => K::GtEq,
                    Opcode::Less => K::Lt,
                    Opcode::LessEqual => K::LtEq,
                    _ => unreachable!(),
                };
                let positions = (
                    pos,
                    Self::position_of_operand(&instruction, 0),
                    Self::position_of_operand(&instruction, 1),
                );
                let expression_typed = typed || left_typed || right_typed;
                let value = runtime_ops::binary(left, operator, right, positions)
                    .map_err(|error| self.operation_error(frame, expression_typed, error))?;
                self.push(value, expression_typed);
            }
            Opcode::Jump(target) => self.frames[frame_index].ip = target as usize,
            Opcode::JumpIfFalse(target) => {
                let (value, value_typed) = self.pop(frame, &instruction)?;
                match value {
                    V::Bool(false) => self.frames[frame_index].ip = target as usize,
                    V::Bool(true) => {}
                    other => {
                        return Err(self.type_error(
                            frame,
                            typed || value_typed,
                            format!("Condition must be Boolean, received {}.", other.ty()),
                            pos,
                        ))
                    }
                }
            }
            Opcode::Call(count) => self.call(frame, &instruction, count as usize)?,
            Opcode::BuildArray(count) => {
                let mut values = Vec::with_capacity(count as usize);
                let mut array_typed = false;
                for _ in 0..count {
                    let (value, typed_value) = self.pop(frame, &instruction)?;
                    array_typed |= typed_value;
                    values.push(value);
                }
                values.reverse();
                self.push(V::Arr(Rc::new(RefCell::new(values))), array_typed);
            }
            Opcode::GetIndex => {
                let (index_value, index_typed) = self.pop(frame, &instruction)?;
                let (receiver, receiver_typed) = self.pop(frame, &instruction)?;
                let index_pos = Self::position_of_operand(&instruction, 1);
                let index =
                    self.array_index(frame, index_value, typed || index_typed, index_pos)?;
                match receiver {
                    V::Arr(items) => {
                        let value = items.borrow().get(index).cloned().ok_or_else(|| {
                            self.error(
                                frame,
                                "E3000",
                                format!("Array index {index} is out of bounds."),
                                pos,
                            )
                        })?;
                        self.push(value, receiver_typed || index_typed);
                    }
                    _other => {
                        return Err(self.type_error(
                            frame,
                            typed || receiver_typed,
                            "Indexing requires Array.",
                            Self::position_of_operand(&instruction, 0),
                        ))
                    }
                }
            }
            Opcode::SetIndex => {
                let (value, value_typed) = self.pop(frame, &instruction)?;
                let (index_value, index_typed) = self.pop(frame, &instruction)?;
                let (receiver, receiver_typed) = self.pop(frame, &instruction)?;
                let index_pos = Self::position_of_operand(&instruction, 1);
                let index =
                    self.array_index(frame, index_value, typed || index_typed, index_pos)?;
                match receiver {
                    V::Arr(items) => {
                        if index >= items.borrow().len() {
                            return Err(self.error(
                                frame,
                                "E3000",
                                format!("Array index {index} is out of bounds."),
                                pos,
                            ));
                        }
                        let identity = Rc::as_ptr(&items) as usize;
                        if contains_array_reference(
                            &value,
                            identity,
                            &mut std::collections::HashSet::new(),
                        ) {
                            return Err(self.error(
                                frame,
                                "E3000",
                                "Cyclic array references are not supported in HK 0.1.x.",
                                pos,
                            ));
                        }
                        items.borrow_mut()[index] = value.clone();
                        self.push(value, value_typed || receiver_typed || index_typed);
                    }
                    _ => {
                        return Err(self.type_error(
                            frame,
                            typed || receiver_typed,
                            "Indexed assignment requires Array.",
                            Self::position_of_operand(&instruction, 0),
                        ))
                    }
                }
            }
            Opcode::GetProperty(name_index) => {
                let name = self.constant_string(frame, name_index, &instruction)?;
                let (receiver, receiver_typed) = self.pop(frame, &instruction)?;
                if name == "length" {
                    match receiver {
                        V::Arr(items) => {
                            self.push(V::Num(items.borrow().len() as f64), receiver_typed)
                        }
                        other => {
                            return Err(self.type_error(
                                frame,
                                typed || receiver_typed,
                                format!(
                                    "Property 'length' requires Array, received {}.",
                                    other.ty()
                                ),
                                Self::position_of_operand(&instruction, 0),
                            ))
                        }
                    }
                } else {
                    return Err(self.error(
                        frame,
                        "E3000",
                        format!("Property '{name}' is not available on {}.", receiver.ty()),
                        pos,
                    ));
                }
            }
            Opcode::Return => {
                let (value, value_typed) = self.pop(frame, &instruction)?;
                let frame = self.frames.pop().expect("active frame");
                let prototype = &self.program.functions[frame.function as usize];
                if let Some(expected) = prototype.return_contract {
                    let return_pos = Self::position_of_operand(&instruction, 0);
                    ensure_contract(expected, &value, "function return").map_err(|message| {
                        HkError::new(
                            "E3008",
                            message,
                            &frame.source,
                            return_pos.0,
                            return_pos.1,
                            &frame.source_text,
                        )
                    })?;
                }
                self.stack.truncate(frame.stack_base);
                self.typed_stack.truncate(frame.stack_base);
                if self.frames.is_empty() {
                    return Ok(());
                }
                self.push(
                    value,
                    value_typed || prototype.typed || prototype.return_contract.is_some(),
                );
            }
            Opcode::DefineModule(_)
            | Opcode::GetModule(_)
            | Opcode::SetModule(_)
            | Opcode::GetImport(_) => {
                return Err(self.error(
                    frame,
                    "E4008",
                    "Module bytecode requires the module VM linker.",
                    pos,
                ));
            }
        }

        debug_assert_eq!(self.stack.len(), self.typed_stack.len());
        if let Some(frame) = self.frames.last() {
            let height = self.stack.len().saturating_sub(frame.stack_base);
            let maximum = self.program.functions[frame.function as usize].max_stack as usize;
            if height > maximum {
                return Err(self.error(
                    frame,
                    "E4009",
                    "Verified bytecode exceeded its declared operand-stack limit.",
                    pos,
                ));
            }
        }
        Ok(())
    }

    fn array_index(
        &self,
        frame: &Frame,
        value: V,
        typed: bool,
        position: (usize, usize),
    ) -> Result<usize> {
        match value {
            V::Num(number) => {
                if number >= 0.0 && number.fract() == 0.0 && number < usize::MAX as f64 {
                    Ok(number as usize)
                } else {
                    Err(self.error(
                        frame,
                        "E3000",
                        "Array index must be a non-negative integer.",
                        position,
                    ))
                }
            }
            _other => Err(self.type_error(frame, typed, "Array index must be Number.", position)),
        }
    }

    fn operation_error(&self, frame: &Frame, typed: bool, error: RuntimeOpError) -> HkError {
        match error {
            RuntimeOpError::Type { message, position } => {
                self.type_error(frame, typed, message, position)
            }
            RuntimeOpError::Runtime { message, position } => {
                self.error(frame, "E3000", message, position)
            }
        }
    }

    fn run_loop(&mut self) -> Result<()> {
        while !self.frames.is_empty() {
            let frame_index = self.frames.len() - 1;
            let function_index = self.frames[frame_index].function;
            let ip = self.frames[frame_index].ip;
            let instruction = self.program.functions[function_index as usize]
                .code
                .get(ip)
                .cloned()
                .ok_or_else(|| {
                    let frame = &self.frames[frame_index];
                    self.error(
                        frame,
                        "E4009",
                        "Verified function fell off the end without RETURN.",
                        (1, 1),
                    )
                })?;
            self.frames[frame_index].ip += 1;
            self.execute_instruction(instruction)?;
        }
        Ok(())
    }

    fn run(mut self) -> Result<Vec<String>> {
        self.run_loop()?;
        Ok(self.output)
    }
}

fn position(instruction: &Instruction) -> (usize, usize) {
    (
        instruction.source.line as usize,
        instruction.source.column as usize,
    )
}

pub(super) fn execute(
    verified: VerifiedProgram,
    source_name: &str,
    source_text: &str,
) -> Result<Vec<String>> {
    Vm::new(verified.program(), source_name, source_text).run()
}

pub(super) fn execute_modules(
    verified: VerifiedProgram,
    initializers: &[u32],
    modules: &[(String, String, super::runtime::ModuleEnv)],
) -> Result<Vec<String>> {
    let program = verified.program();
    if initializers.len() != modules.len() {
        return Err(HkError::new(
            "E4009",
            "Module initializer table does not match the linked module graph.",
            "<module-graph>",
            1,
            1,
            "",
        ));
    }
    let mut vm = Vm::new_empty(program);
    for ((source_name, source_text, environment), function) in modules.iter().zip(initializers) {
        let function_proto = program.functions.get(*function as usize).ok_or_else(|| {
            HkError::new(
                "E4009",
                "Module initializer references a missing bytecode function.",
                source_name,
                1,
                1,
                source_text,
            )
        })?;
        Env::ensure_print(environment);
        vm.frames.push(Frame {
            function: *function,
            ip: 0,
            env: environment.clone(),
            scopes: vec![environment.clone()],
            closure_env: None,
            stack_base: 0,
            source: Rc::new(source_name.clone()),
            source_text: Rc::new(source_text.clone()),
            typed: function_proto.typed,
        });
        vm.run_loop()?;
        debug_assert!(vm.stack.is_empty());
        debug_assert!(vm.typed_stack.is_empty());
    }
    Ok(vm.output)
}

pub(super) fn run_source(source_name: &str, source_text: &str) -> Result<Vec<String>> {
    let verified = super::compiler::compile_source(source_name, source_text)?;
    execute(verified, source_name, source_text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;
    use crate::parser::Parser;

    fn reference(source: &str) -> Result<Vec<String>> {
        let name = "vm-differential.hk";
        let tokens = Lexer::new(name, source).scan()?;
        let statements = Parser::new(tokens, name, source).parse()?;
        if crate::has_type_annotations(&statements) {
            crate::type_checker::TypeChecker::new(name, source).check(&statements)?;
        }
        super::super::runtime::execute(name, source, &statements)
    }

    fn assert_matches_reference(source: &str) {
        let expected = reference(source);
        let actual = run_source("vm-differential.hk", source);
        match (expected, actual) {
            (Ok(expected), Ok(actual)) => assert_eq!(actual, expected, "source:\n{source}"),
            (Err(expected), Err(actual)) => {
                assert_eq!(actual.code, expected.code, "source:\n{source}");
                assert_eq!(actual.message, expected.message, "source:\n{source}");
                assert_eq!((actual.line, actual.column), (expected.line, expected.column), "source:\n{source}");
            }
            (expected, actual) => panic!("reference/VM outcome mismatch for:\n{source}\nreference={expected:?}\nvm={actual:?}"),
        }
    }

    #[test]
    fn vm_matches_reference_for_values_arrays_and_control_flow() {
        assert_matches_reference(
            r#"
            let values = [1, 2, 3];
            values[1] = values[0] + 4;
            let total = values[0] + values[1] + values.length;
            if (total > 7) { print(total); } else { print(0); }
            let i = 0;
            while (i < 2) { print(values[i]); i = i + 1; }
            for (let j = 0; j < 2; j = j + 1) { print(j); }
        "#,
        );
    }

    #[test]
    fn vm_matches_reference_for_nested_mutable_and_transitive_captures() {
        assert_matches_reference(
            r#"
            fn outer() {
                let value = 1;
                fn middle() {
                    fn inner() { value = value + 1; print(value); }
                    inner();
                }
                middle();
                print(value);
            }
            outer();
        "#,
        );
    }

    #[test]
    fn vm_matches_reference_for_recursive_and_mutually_recursive_functions() {
        assert_matches_reference(
            r#"
            fn even(n) {
                if (n == 0) { return true; }
                return odd(n - 1);
            }
            fn odd(n) {
                if (n == 0) { return false; }
                return even(n - 1);
            }
            print(even(8));
            print(odd(7));
        "#,
        );
    }

    #[test]
    fn vm_matches_reference_for_short_circuit_and_dynamic_names() {
        assert_matches_reference(
            r#"
            let value = false && print("not evaluated");
            let other = true || print("not evaluated");
            let print_alias = print;
            print_alias(value);
            print(other);
        "#,
        );
    }

    #[test]
    fn vm_matches_reference_for_runtime_errors_and_weak_environment_expiration() {
        assert_matches_reference("let value = 1; value = \"bad\";");
        assert_matches_reference("let value = [1]; print(value[2]);");
        assert_matches_reference("fn outer() { let value = 1; fn inner() { print(value); } return inner; } let f = outer(); f();");
        assert_matches_reference("let value = 1; value();");
    }

    #[test]
    fn vm_preserves_dynamic_contract_provenance_through_operations() {
        assert_matches_reference("let value: any = 1; print(value + \"bad\");");
        assert_matches_reference("let condition: any = 1; if (condition) { print(1); }");
        assert_matches_reference("let value: any = 1; print(!value);");
        assert_matches_reference("let value: any = 1; print(value.length);");
        assert_matches_reference("let dynamic: any = \"bad\"; let value: number = dynamic;");
        assert_matches_reference("fn identity(value: any) -> number { return value; } let dynamic: any = \"bad\"; print(identity(dynamic));");
        assert_matches_reference("let values: Array = [1]; print(values[0] + \"bad\");");
        assert_matches_reference(
            "let value: any = 1; fn read() { return value; } print(read() + \"bad\");",
        );
    }

    #[test]
    fn vm_matches_reference_for_left_to_right_side_effects_and_array_identity() {
        assert_matches_reference(
            r#"
            let order = 0;
            fn mark(value) { order = order * 10 + value; return value; }
            let values = [0, 0];
            values[mark(1)] = mark(2);
            print(order);
            print(values[1]);
            let shared = values;
            shared[0] = 9;
            print(values[0]);
            print(shared == values);
            print([1] == [1]);
        "#,
        );
    }

    #[test]
    fn vm_matches_reference_for_shadowing_const_mutation_and_null_fallthrough() {
        assert_matches_reference(
            r#"
            let value = 1;
            {
                let value = 2;
                print(value);
            }
            print(value);
            const items = [1];
            items[0] = 3;
            print(items[0]);
            fn no_return() {}
            print(no_return());
            print(null == null);
        "#,
        );
    }

    #[test]
    fn vm_matches_reference_for_gradual_contract_boundaries() {
        assert_matches_reference("let value: number = 1; value = \"bad\";");
        assert_matches_reference(
            "fn identity(value: number) -> number { return value; } print(identity(\"bad\"));",
        );
        assert_matches_reference("fn identity(value: number) -> number { return value; } let dynamic = identity; print(dynamic(1));");
    }
}
