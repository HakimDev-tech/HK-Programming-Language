#![allow(dead_code)] // Keep compiler internals private; only verified compilation entry points are exposed.
//! Deterministic AST-to-bytecode lowering for source-only HK programs.
use super::ast::{TypeName, E, S};
use super::bytecode::{
    BytecodeHeader, BytecodeProgram, Constant, FunctionProto, Instruction, LocalSlot, Opcode,
    ParameterProto, SourceFile, SourceLocation, UpvalueDescriptor, UpvalueSource,
};
use super::diagnostics::HkError;
use super::lexer::{Lit, K};
use super::parser::Parser;
use super::types::Type;
use super::verifier::{self, VerifiedProgram};
use super::{has_type_annotations, type_checker, Result};
use std::collections::HashMap;

#[derive(Clone, Copy, Debug)]
enum BindingRef {
    Local(u16),
    Upvalue(u16),
}

#[derive(Clone, Copy, Debug)]
struct ResolvedBinding {
    reference: BindingRef,
    mutable: bool,
    contract: Option<Type>,
}

struct FunctionBuilder {
    index: u32,
    name: String,
    parent: Option<u32>,
    parameters: Vec<ParameterProto>,
    local_slots: Vec<LocalSlot>,
    upvalues: Vec<UpvalueDescriptor>,
    scopes: Vec<HashMap<String, u16>>,
    typed_functions: Vec<HashMap<String, bool>>,
    code: Vec<Instruction>,
    typed: bool,
    return_contract: Option<Type>,
    typed_context: bool,
    scope_depth: u32,
}

impl FunctionBuilder {
    fn new(
        index: u32,
        name: String,
        parent: Option<u32>,
        typed: bool,
        return_contract: Option<Type>,
    ) -> Self {
        Self {
            index,
            name,
            parent,
            parameters: Vec::new(),
            local_slots: Vec::new(),
            upvalues: Vec::new(),
            scopes: vec![HashMap::new()],
            typed_functions: vec![HashMap::new()],
            code: Vec::new(),
            typed,
            return_contract,
            typed_context: typed,
            scope_depth: 0,
        }
    }

    fn local(&self, name: &str) -> Option<u16> {
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| scope.get(name).copied())
    }

    fn allocate_local(&mut self, name: &str, mutable: bool, contract: Option<Type>) -> u16 {
        let slot = self.local_slots.len() as u16;
        self.local_slots.push(LocalSlot {
            internal_name: format!("\\0{}:{slot}", self.index),
            source_name: name.to_owned(),
            mutable,
            contract,
        });
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name.to_owned(), slot);
        }
        if let Some(scope) = self.typed_functions.last_mut() {
            scope.insert(name.to_owned(), false);
        }
        slot
    }
}

#[derive(Hash, PartialEq, Eq)]
enum ConstantKey {
    Number(u64),
    String(String),
}

struct Compiler<'a> {
    source_name: &'a str,
    source_text: &'a str,
    constants: Vec<Constant>,
    constant_indices: HashMap<ConstantKey, u32>,
    functions: Vec<Option<FunctionProto>>,
    builders: Vec<FunctionBuilder>,
    allow_imports: bool,
}

impl<'a> Compiler<'a> {
    fn new(source_name: &'a str, source_text: &'a str) -> Self {
        let mut compiler = Self {
            source_name,
            source_text,
            constants: Vec::new(),
            constant_indices: HashMap::new(),
            functions: Vec::new(),
            builders: Vec::new(),
            allow_imports: false,
        };
        compiler.functions.push(None);
        compiler.builders.push(FunctionBuilder::new(
            0,
            "<entry>".to_owned(),
            None,
            false,
            None,
        ));
        compiler
    }

    fn error(&self, message: impl Into<String>, line: usize, column: usize) -> HkError {
        HkError::new(
            "E4009",
            message,
            self.source_name,
            line,
            column,
            self.source_text,
        )
    }

    fn constant(&mut self, value: Constant) -> u32 {
        let key = match &value {
            Constant::Number(number) => ConstantKey::Number(number.to_bits()),
            Constant::String(string) => ConstantKey::String(string.clone()),
        };
        if let Some(index) = self.constant_indices.get(&key) {
            return *index;
        }
        let index = self.constants.len() as u32;
        self.constants.push(value);
        self.constant_indices.insert(key, index);
        index
    }

    fn string_constant(&mut self, value: &str) -> u32 {
        self.constant(Constant::String(value.to_owned()))
    }

    fn emit(
        &mut self,
        opcode: Opcode,
        position: (usize, usize),
        operands: &[(usize, usize)],
    ) -> usize {
        let builder = self.builders.last().expect("active function builder");
        let instruction = Instruction {
            opcode,
            source: SourceLocation {
                source_id: 0,
                line: position.0.max(1) as u32,
                column: position.1.max(1) as u32,
            },
            operands: operands
                .iter()
                .map(|(line, column)| SourceLocation {
                    source_id: 0,
                    line: (*line).max(1) as u32,
                    column: (*column).max(1) as u32,
                })
                .collect(),
            typed_context: builder.typed_context,
        };
        let code = &mut self
            .builders
            .last_mut()
            .expect("active function builder")
            .code;
        code.push(instruction);
        code.len() - 1
    }

    fn patch_jump(&mut self, instruction: usize, target: usize) {
        let opcode = &mut self
            .builders
            .last_mut()
            .expect("active function builder")
            .code[instruction]
            .opcode;
        match opcode {
            Opcode::Jump(address) | Opcode::JumpIfFalse(address) => *address = target as u32,
            _ => unreachable!("only jump instructions are patched"),
        }
    }

    fn mark_typed_function(&mut self, name: &str) {
        if let Some(scope) = self
            .builders
            .last_mut()
            .and_then(|builder| builder.typed_functions.last_mut())
        {
            scope.insert(name.to_owned(), true);
        }
    }

    fn is_typed_function_binding(&self, depth: usize, name: &str) -> bool {
        for builder in self.builders[..=depth].iter().rev() {
            for scope in builder.typed_functions.iter().rev() {
                if let Some(typed) = scope.get(name) {
                    return *typed;
                }
            }
        }
        false
    }

    fn define_local(&mut self, name: &str, mutable: bool, contract: Option<Type>) -> u16 {
        self.builders
            .last_mut()
            .expect("active function builder")
            .allocate_local(name, mutable, contract)
    }

    fn resolve_binding(&mut self, depth: usize, name: &str) -> Option<ResolvedBinding> {
        if let Some(slot) = self.builders[depth].local(name) {
            let local = &self.builders[depth].local_slots[slot as usize];
            return Some(ResolvedBinding {
                reference: BindingRef::Local(slot),
                mutable: local.mutable,
                contract: local.contract,
            });
        }
        if depth == 0 {
            return None;
        }

        let parent = self.resolve_binding(depth - 1, name)?;
        let source = match parent.reference {
            BindingRef::Local(slot) => UpvalueSource::EnclosingLocal(slot),
            BindingRef::Upvalue(slot) => UpvalueSource::EnclosingUpvalue(slot),
        };
        let upvalues = &mut self.builders[depth].upvalues;
        let slot =
            if let Some(existing) = upvalues.iter().position(|capture| capture.source == source) {
                existing as u16
            } else {
                upvalues.push(UpvalueDescriptor {
                    source,
                    mutable: parent.mutable,
                    contract: parent.contract,
                });
                (upvalues.len() - 1) as u16
            };
        Some(ResolvedBinding {
            reference: BindingRef::Upvalue(slot),
            mutable: parent.mutable,
            contract: parent.contract,
        })
    }

    fn compile_statements(&mut self, statements: &[S]) -> Result<()> {
        // Module-entry functions are hoisted: allocate all slots and compile
        // closures before ordinary initializers, making forward calls executable.
        let is_module_entry = self
            .builders
            .last()
            .is_some_and(|builder| builder.index == 0 && builder.scope_depth == 0);
        if is_module_entry {
            // Allocate every binding before compiling any closure, so each
            // function can resolve all forward and mutually recursive peers.
            let mut declarations = Vec::new();
            for statement in statements {
                let declaration = match statement {
                    S::Export(inner) => inner.as_ref(),
                    other => other,
                };
                match declaration {
                    S::Fn(name, ..) | S::TypedFn(name, ..) => {
                        let slot = self.define_local(name, false, None);
                        declarations.push((statement, slot));
                    }
                    _ => {}
                }
            }

            for (statement, slot) in declarations {
                let declaration = match statement {
                    S::Export(inner) => inner.as_ref(),
                    other => other,
                };
                match declaration {
                    S::Fn(name, parameters, body, line, column) => {
                        let child = self.compile_function(
                            name,
                            parameters
                                .iter()
                                .map(|parameter| (parameter.as_str(), None))
                                .collect(),
                            body,
                            false,
                            None,
                        )?;
                        self.emit(Opcode::MakeFunction(child), (*line, *column), &[]);
                        self.emit(Opcode::DefineFunctionLocal(slot), (*line, *column), &[]);
                    }
                    S::TypedFn(name, parameters, return_type, body, line, column) => {
                        self.mark_typed_function(name);
                        let params = parameters
                            .iter()
                            .map(|parameter| (parameter.name.as_str(), parameter.annotation))
                            .collect();
                        let child = self.compile_function(
                            name,
                            params,
                            body,
                            true,
                            return_type.map(Type::from),
                        )?;
                        self.emit(Opcode::MakeFunction(child), (*line, *column), &[]);
                        self.emit(Opcode::DefineFunctionLocal(slot), (*line, *column), &[]);
                    }
                    _ => continue,
                }
            }
        }

        for statement in statements {
            if is_module_entry
                && matches!(
                    match statement {
                        S::Export(inner) => inner.as_ref(),
                        other => other,
                    },
                    S::Fn(..) | S::TypedFn(..)
                )
            {
                continue;
            }
            self.compile_statement(statement)?;
        }
        Ok(())
    }

    fn compile_statement(&mut self, statement: &S) -> Result<()> {
        match statement {
            S::Import(_, _, line, column) => {
                if self.allow_imports {
                    return Ok(());
                }
                return Err(HkError::new(
                    "E4008",
                    "Filesystem imports require the module compiler and hk::run_path().",
                    self.source_name,
                    *line,
                    *column,
                    self.source_text,
                ));
            }
            S::Export(inner) => self.compile_statement(inner)?,
            S::Var(name, initializer, mutable, line, column) => {
                self.compile_expression(initializer)?;
                let slot = self.define_local(name, *mutable, None);
                self.emit(
                    Opcode::DefineLocal(slot),
                    (*line, *column),
                    &[initializer.source_position()],
                );
            }
            S::TypedVar(name, annotation, initializer, mutable, line, column) => {
                let previous = self.builders.last().unwrap().typed_context;
                self.builders.last_mut().unwrap().typed_context = true;
                self.compile_expression(initializer)?;
                self.builders.last_mut().unwrap().typed_context = previous;
                let slot = self.define_local(name, *mutable, Some(Type::from(*annotation)));
                self.emit(
                    Opcode::DefineLocal(slot),
                    (*line, *column),
                    &[initializer.source_position()],
                );
            }
            S::Fn(name, parameters, body, line, column) => {
                let slot = self.define_local(name, false, None);
                let child = self.compile_function(
                    name,
                    parameters
                        .iter()
                        .map(|name| (name.as_str(), None))
                        .collect(),
                    body,
                    false,
                    None,
                )?;
                self.emit(Opcode::MakeFunction(child), (*line, *column), &[]);
                self.emit(Opcode::DefineFunctionLocal(slot), (*line, *column), &[]);
            }
            S::TypedFn(name, parameters, return_type, body, line, column) => {
                let slot = self.define_local(name, false, None);
                self.mark_typed_function(name);
                let params = parameters
                    .iter()
                    .map(|parameter| (parameter.name.as_str(), parameter.annotation))
                    .collect();
                let child =
                    self.compile_function(name, params, body, true, return_type.map(Type::from))?;
                self.emit(Opcode::MakeFunction(child), (*line, *column), &[]);
                self.emit(Opcode::DefineFunctionLocal(slot), (*line, *column), &[]);
            }
            S::Expr(expression) => {
                self.compile_expression(expression)?;
                let (line, column) = expression.source_position();
                self.emit(Opcode::Pop, (line, column), &[]);
            }
            S::Block(statements) => {
                let position = statements.first().map(S::source_position).unwrap_or((1, 1));
                self.emit(Opcode::EnterScope, position, &[]);
                let builder = self.builders.last_mut().unwrap();
                builder.scopes.push(HashMap::new());
                builder.typed_functions.push(HashMap::new());
                builder.scope_depth += 1;
                self.compile_statements(statements)?;
                self.emit(Opcode::ExitScope, position, &[]);
                let builder = self.builders.last_mut().unwrap();
                builder.scopes.pop();
                builder.typed_functions.pop();
                builder.scope_depth -= 1;
            }
            S::If(condition, then_branch, else_branch) => {
                self.compile_expression(condition)?;
                let condition_position = condition.source_position();
                let branch = self.emit(Opcode::JumpIfFalse(0), condition_position, &[]);
                self.compile_statement(then_branch)?;
                if let Some(else_branch) = else_branch {
                    let skip_else = self.emit(Opcode::Jump(0), then_branch.source_position(), &[]);
                    self.patch_jump(branch, self.builders.last().unwrap().code.len());
                    self.compile_statement(else_branch)?;
                    self.patch_jump(skip_else, self.builders.last().unwrap().code.len());
                } else {
                    self.patch_jump(branch, self.builders.last().unwrap().code.len());
                }
            }
            S::While(condition, body) => {
                let start = self.builders.last().unwrap().code.len();
                self.compile_expression(condition)?;
                let position = condition.source_position();
                let exit = self.emit(Opcode::JumpIfFalse(0), position, &[]);
                self.compile_statement(body)?;
                self.emit(Opcode::Jump(start as u32), position, &[]);
                self.patch_jump(exit, self.builders.last().unwrap().code.len());
            }
            S::For(initializer, condition, increment, body) => {
                let position = initializer
                    .as_ref()
                    .map(|item| item.source_position())
                    .or_else(|| condition.as_ref().map(E::source_position))
                    .unwrap_or_else(|| body.source_position());
                self.emit(Opcode::EnterScope, position, &[]);
                self.builders
                    .last_mut()
                    .unwrap()
                    .scopes
                    .push(HashMap::new());
                self.builders.last_mut().unwrap().scope_depth += 1;
                if let Some(initializer) = initializer {
                    self.compile_statement(initializer)?;
                }
                let start = self.builders.last().unwrap().code.len();
                let exit = if let Some(condition) = condition {
                    self.compile_expression(condition)?;
                    Some(self.emit(Opcode::JumpIfFalse(0), condition.source_position(), &[]))
                } else {
                    None
                };
                self.compile_statement(body)?;
                if let Some(increment) = increment {
                    self.compile_expression(increment)?;
                    self.emit(Opcode::Pop, increment.source_position(), &[]);
                }
                self.emit(Opcode::Jump(start as u32), position, &[]);
                if let Some(exit) = exit {
                    self.patch_jump(exit, self.builders.last().unwrap().code.len());
                } else {
                    // An unconditional loop has no reachable fallthrough; retain a
                    // verifier-visible exit edge for the scope-unwind instruction.
                    let jump = self.emit(Opcode::Jump(0), position, &[]);
                    self.patch_jump(jump, self.builders.last().unwrap().code.len() + 1);
                }
                self.emit(Opcode::ExitScope, position, &[]);
                self.builders.last_mut().unwrap().scopes.pop();
                self.builders.last_mut().unwrap().typed_functions.pop();
                self.builders.last_mut().unwrap().scope_depth -= 1;
            }
            S::Return(expression, line, column) => {
                if self.builders.len() == 1 {
                    return Err(HkError::new(
                        "E3000",
                        "'return' is only valid inside a function.",
                        self.source_name,
                        *line,
                        *column,
                        self.source_text,
                    ));
                }
                let return_operand = if let Some(expression) = expression {
                    self.compile_expression(expression)?;
                    vec![expression.source_position()]
                } else {
                    self.emit(Opcode::Null, (*line, *column), &[]);
                    Vec::new()
                };
                let depth = self.builders.last().unwrap().scope_depth;
                for _ in 0..depth {
                    self.emit(Opcode::ExitScope, (*line, *column), &[]);
                }
                self.emit(Opcode::Return, (*line, *column), &return_operand);
            }
        }
        Ok(())
    }

    fn compile_function(
        &mut self,
        name: &str,
        parameters: Vec<(&str, Option<TypeName>)>,
        body: &[S],
        typed: bool,
        return_contract: Option<Type>,
    ) -> Result<u32> {
        let parent = self.builders.last().unwrap().index;
        let index = self.functions.len() as u32;
        self.functions.push(None);
        let mut builder =
            FunctionBuilder::new(index, name.to_owned(), Some(parent), typed, return_contract);
        for (parameter_name, annotation) in parameters {
            let slot = builder.allocate_local(parameter_name, true, annotation.map(Type::from));
            builder.parameters.push(ParameterProto {
                name: parameter_name.to_owned(),
                slot,
                annotation,
            });
        }
        self.builders.push(builder);
        self.compile_statements(body)?;
        let position = body.last().map(S::source_position).unwrap_or((1, 1));
        let depth = self.builders.last().unwrap().scope_depth;
        for _ in 0..depth {
            self.emit(Opcode::ExitScope, position, &[]);
        }
        self.emit(Opcode::Null, position, &[]);
        self.emit(Opcode::Return, position, &[]);
        let builder = self.builders.pop().unwrap();
        self.functions[index as usize] = Some(FunctionProto {
            name: builder.name,
            module_id: None,
            parameters: builder.parameters,
            local_slots: builder.local_slots,
            enclosing_function: builder.parent,
            upvalues: builder.upvalues,
            return_contract: builder.return_contract,
            typed: builder.typed,
            max_stack: 0,
            code: builder.code,
        });
        Ok(index)
    }

    fn compile_binary_chain(&mut self, expression: &E) -> Result<()> {
        let mut chain = Vec::new();
        let mut cursor = expression;
        while let E::Binary(left, operator, right, line, column) = cursor {
            chain.push((
                *operator,
                right.as_ref(),
                (*line, *column),
                left.source_position(),
                right.source_position(),
            ));
            cursor = left.as_ref();
        }
        self.compile_expression(cursor)?;
        for (operator, right, position, left_position, right_position) in chain.into_iter().rev() {
            match operator {
                K::And => {
                    self.emit(Opcode::Dup, position, &[]);
                    let exit = self.emit(Opcode::JumpIfFalse(0), left_position, &[]);
                    self.emit(Opcode::Pop, position, &[]);
                    self.compile_expression(right)?;
                    self.emit(Opcode::AssertBoolean, right_position, &[]);
                    self.patch_jump(exit, self.builders.last().unwrap().code.len());
                }
                K::Or => {
                    self.emit(Opcode::Dup, position, &[]);
                    let evaluate_right = self.emit(Opcode::JumpIfFalse(0), left_position, &[]);
                    let exit = self.emit(Opcode::Jump(0), position, &[]);
                    self.patch_jump(evaluate_right, self.builders.last().unwrap().code.len());
                    self.emit(Opcode::Pop, position, &[]);
                    self.compile_expression(right)?;
                    self.emit(Opcode::AssertBoolean, right_position, &[]);
                    self.patch_jump(exit, self.builders.last().unwrap().code.len());
                }
                _ => {
                    self.compile_expression(right)?;
                    let opcode = match operator {
                        K::Plus => Opcode::Add,
                        K::Minus => Opcode::Subtract,
                        K::Star => Opcode::Multiply,
                        K::Slash => Opcode::Divide,
                        K::Percent => Opcode::Modulo,
                        K::EqEq => Opcode::Equal,
                        K::BangEq => Opcode::NotEqual,
                        K::Gt => Opcode::Greater,
                        K::GtEq => Opcode::GreaterEqual,
                        K::Lt => Opcode::Less,
                        K::LtEq => Opcode::LessEqual,
                        _ => {
                            return Err(self.error(
                                "Unsupported binary operator in AST.",
                                position.0,
                                position.1,
                            ))
                        }
                    };
                    self.emit(opcode, position, &[left_position, right_position]);
                }
            }
        }
        Ok(())
    }

    fn compile_expression(&mut self, expression: &E) -> Result<()> {
        match expression {
            E::Lit(Lit::Number(number), line, column) => {
                let index = self.constant(Constant::Number(*number));
                self.emit(Opcode::Constant(index), (*line, *column), &[]);
            }
            E::Lit(Lit::String(string), line, column) => {
                let index = self.string_constant(string);
                self.emit(Opcode::Constant(index), (*line, *column), &[]);
            }
            E::Bool(value, line, column) => {
                self.emit(
                    if *value { Opcode::True } else { Opcode::False },
                    (*line, *column),
                    &[],
                );
            }
            E::Null(line, column) => {
                self.emit(Opcode::Null, (*line, *column), &[]);
            }
            E::Var(name, line, column) => {
                if name == "print"
                    && self
                        .resolve_binding(self.builders.len() - 1, name)
                        .is_none()
                {
                    self.emit(Opcode::LoadPrint, (*line, *column), &[]);
                } else if let Some(binding) = self.resolve_binding(self.builders.len() - 1, name) {
                    let opcode = match binding.reference {
                        BindingRef::Local(slot) => Opcode::GetLocal(slot),
                        BindingRef::Upvalue(slot) => Opcode::GetUpvalue(slot),
                    };
                    self.emit(opcode, (*line, *column), &[]);
                } else {
                    let name_index = self.string_constant(name);
                    self.emit(Opcode::GetName(name_index), (*line, *column), &[]);
                }
            }
            E::Assign(name, value, line, column) => {
                self.compile_expression(value)?;
                let opcode = match self.resolve_binding(self.builders.len() - 1, name) {
                    Some(ResolvedBinding {
                        reference: BindingRef::Local(slot),
                        ..
                    }) => Opcode::SetLocal(slot),
                    Some(ResolvedBinding {
                        reference: BindingRef::Upvalue(slot),
                        ..
                    }) => Opcode::SetUpvalue(slot),
                    None => {
                        let name_index = self.string_constant(name);
                        Opcode::SetName(name_index)
                    }
                };
                self.emit(opcode, (*line, *column), &[value.source_position()]);
            }
            E::Unary(operator, operand, line, column) => {
                self.compile_expression(operand)?;
                let opcode = match operator {
                    K::Minus => Opcode::Negate,
                    K::Bang => Opcode::Not,
                    _ => {
                        return Err(self.error(
                            "Unsupported unary operator in AST.",
                            *line,
                            *column,
                        ))
                    }
                };
                self.emit(opcode, (*line, *column), &[operand.source_position()]);
            }
            E::Binary(..) => self.compile_binary_chain(expression)?,
            E::Group(inner) => self.compile_expression(inner)?,
            E::Call(callee, arguments, line, column) => {
                self.compile_expression(callee)?;
                let typed_direct_call = if let E::Var(name, _, _) = callee.as_ref() {
                    self.is_typed_function_binding(self.builders.len() - 1, name)
                } else {
                    false
                };
                let previous_context = self.builders.last().unwrap().typed_context;
                if typed_direct_call {
                    self.builders.last_mut().unwrap().typed_context = true;
                }
                for argument in arguments {
                    self.compile_expression(argument)?;
                }
                self.builders.last_mut().unwrap().typed_context = previous_context;
                let mut operands = Vec::with_capacity(arguments.len() + 1);
                operands.push(callee.source_position());
                operands.extend(arguments.iter().map(E::source_position));
                self.emit(
                    Opcode::Call(arguments.len() as u16),
                    (*line, *column),
                    &operands,
                );
            }
            E::Array(elements, line, column) => {
                for element in elements {
                    self.compile_expression(element)?;
                }
                self.emit(
                    Opcode::BuildArray(elements.len() as u16),
                    (*line, *column),
                    &[],
                );
            }
            E::Index(receiver, index, line, column) => {
                self.compile_expression(receiver)?;
                self.compile_expression(index)?;
                self.emit(
                    Opcode::GetIndex,
                    (*line, *column),
                    &[receiver.source_position(), index.source_position()],
                );
            }
            E::SetIndex(receiver, index, value, line, column) => {
                self.compile_expression(receiver)?;
                self.compile_expression(index)?;
                self.compile_expression(value)?;
                self.emit(
                    Opcode::SetIndex,
                    (*line, *column),
                    &[
                        receiver.source_position(),
                        index.source_position(),
                        value.source_position(),
                    ],
                );
            }
            E::Property(receiver, name, line, column) => {
                self.compile_expression(receiver)?;
                let name_index = self.string_constant(name);
                self.emit(
                    Opcode::GetProperty(name_index),
                    (*line, *column),
                    &[receiver.source_position()],
                );
            }
        }
        Ok(())
    }

    fn finish(mut self) -> Result<VerifiedProgram> {
        self.compile_statements(&[])?;
        let position = (1, 1);
        self.emit(Opcode::Null, position, &[]);
        self.emit(Opcode::Return, position, &[]);
        let entry = self.builders.pop().unwrap();
        self.functions[0] = Some(FunctionProto {
            name: entry.name,
            module_id: None,
            parameters: entry.parameters,
            local_slots: entry.local_slots,
            enclosing_function: None,
            upvalues: entry.upvalues,
            return_contract: None,
            typed: false,
            max_stack: 0,
            code: entry.code,
        });
        let functions: Vec<FunctionProto> = self
            .functions
            .into_iter()
            .map(|function| function.expect("all function prototypes are finalized"))
            .collect();
        let mut header = BytecodeHeader::default();
        if functions
            .iter()
            .any(|function| !function.upvalues.is_empty())
        {
            header.required_features |= 1;
        }
        let program = BytecodeProgram {
            header,
            constants: self.constants,
            source_files: vec![SourceFile {
                name: self.source_name.to_owned(),
            }],
            functions,
            modules: Vec::new(),
            entry_function: 0,
        };
        verifier::verify_compiler_output(program).map_err(|error| {
            HkError::new(
                "E4009",
                error.to_string(),
                self.source_name,
                1,
                1,
                self.source_text,
            )
        })
    }
}

pub(super) fn compile_module(
    source_name: &str,
    source_text: &str,
    statements: &[S],
) -> Result<VerifiedProgram> {
    super::declaration_collector::collect(statements, source_name, source_text)?;
    if has_type_annotations(statements) {
        type_checker::TypeChecker::new(source_name, source_text).check(statements)?;
    }
    let mut compiler = Compiler::new(source_name, source_text);
    compiler.allow_imports = true;
    compiler.compile_statements(statements)?;
    compiler.finish()
}

/// Compile all dependency-ordered modules into one verified function/constant space.
/// Module environments and import aliases remain owned by the existing module linker.
pub(super) fn compile_module_graph(
    modules: &[(String, String, Vec<S>)],
) -> Result<(VerifiedProgram, Vec<u32>)> {
    if modules.is_empty() {
        return Err(HkError::new(
            "E4009",
            "Module graph is empty.",
            "<module-graph>",
            1,
            1,
            "",
        ));
    }
    let mut constants = Vec::new();
    let mut functions = Vec::new();
    let mut source_files = Vec::new();
    let mut initializers = Vec::with_capacity(modules.len());
    let mut required_features = 0u64;
    for (source_name, source_text, statements) in modules {
        let compiled = compile_module(source_name, source_text, statements)?;
        let program = compiled.program();
        let function_offset = functions.len() as u32;
        let constant_offset = constants.len() as u32;
        let source_id = source_files.len() as u32;
        initializers.push(function_offset + program.entry_function);
        required_features |= program.header.required_features;
        constants.extend(program.constants.iter().cloned());
        source_files.push(SourceFile {
            name: source_name.clone(),
        });
        for function in &program.functions {
            let mut function = function.clone();
            if let Some(parent) = function.enclosing_function.as_mut() {
                *parent += function_offset;
            }
            for instruction in &mut function.code {
                instruction.source.source_id = source_id;
                for operand in &mut instruction.operands {
                    operand.source_id = source_id;
                }
                instruction.opcode = match instruction.opcode {
                    Opcode::Constant(index) => Opcode::Constant(index + constant_offset),
                    Opcode::GetName(index) => Opcode::GetName(index + constant_offset),
                    Opcode::SetName(index) => Opcode::SetName(index + constant_offset),
                    Opcode::GetProperty(index) => Opcode::GetProperty(index + constant_offset),
                    Opcode::MakeFunction(index) => Opcode::MakeFunction(index + function_offset),
                    other => other,
                };
            }
            functions.push(function);
        }
    }
    let mut program = BytecodeProgram {
        header: BytecodeHeader {
            required_features,
            ..BytecodeHeader::default()
        },
        constants,
        source_files,
        functions,
        modules: Vec::new(),
        entry_function: initializers[0],
    };
    let verified = verifier::verify_compiler_output(program.clone())
        .map_err(|error| HkError::new("E4009", error.to_string(), "<module-graph>", 1, 1, ""))?;
    // The verifier recomputes stack bounds; retain its canonical verified program.
    program = verified.program().clone();
    Ok((
        verifier::verify_compiler_output(program).map_err(|error| {
            HkError::new("E4009", error.to_string(), "<module-graph>", 1, 1, "")
        })?,
        initializers,
    ))
}

pub(super) fn compile_source(source_name: &str, source_text: &str) -> Result<VerifiedProgram> {
    let tokens = super::lexer::Lexer::new(source_name, source_text).scan()?;
    let statements = Parser::new(tokens, source_name, source_text).parse()?;
    if let Some(S::Import(_, _, line, column)) = statements
        .iter()
        .find(|statement| matches!(statement, S::Import(..)))
    {
        return Err(HkError::new(
            "E4008",
            "Filesystem imports require hk::run_path() with an entry file.",
            source_name,
            *line,
            *column,
            source_text,
        ));
    }
    super::declaration_collector::collect(&statements, source_name, source_text)?;
    if has_type_annotations(&statements) {
        type_checker::TypeChecker::new(source_name, source_text).check(&statements)?;
    }
    let mut compiler = Compiler::new(source_name, source_text);
    compiler.compile_statements(&statements)?;
    compiler.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compiles_arithmetic_and_source_maps() {
        let verified = compile_source("compiler.hk", "let value = 1 + 2; print(value);").unwrap();
        let program = verified.program();
        assert_eq!(program.functions.len(), 1);
        assert!(program.functions[0]
            .code
            .iter()
            .any(|instruction| matches!(instruction.opcode, Opcode::Add)));
        assert!(program.functions[0]
            .code
            .iter()
            .all(|instruction| instruction.source.line > 0));
    }

    #[test]
    fn compiles_nested_function_capture_and_transitive_forwarding() {
        let source = r#"
            fn outer() {
                let value = 1;
                fn middle() {
                    fn inner() { print(value); }
                    inner();
                }
                middle();
            }
            outer();
        "#;
        let verified = compile_source("captures.hk", source).unwrap();
        let program = verified.program();
        assert!(program
            .functions
            .iter()
            .any(|function| !function.upvalues.is_empty()));
        assert!(program.functions.iter().any(|function| function
            .upvalues
            .iter()
            .any(|upvalue| matches!(upvalue.source, UpvalueSource::EnclosingUpvalue(_)))));
    }

    #[test]
    fn compiles_short_circuit_with_control_flow() {
        let verified = compile_source("logic.hk", "let value = false && print(1);").unwrap();
        assert!(verified.program().functions[0]
            .code
            .iter()
            .any(|instruction| matches!(instruction.opcode, Opcode::JumpIfFalse(_))));
    }

    #[test]
    fn compiles_every_source_expression_and_statement_family() {
        let source = r#"
            let text = "hk";
            let flag = !false && true || false;
            let values = [1, 2, 3];
            let first = values[0];
            values[1] = first + 1;
            let length = values.length;
            let result = (first * 2) >= 2;
            if (result) { let shadow = null; } else { let other = text; }
            while (false) { print(text); }
            for (let i = 0; i < 1; i = i + 1) { values[i] = i; }
            fn identity(value) { return value; }
            let answer = identity(values[0]);
            print(answer);
        "#;
        let verified = compile_source("ast-coverage.hk", source).unwrap();
        let program = verified.program();
        assert!(program.functions.len() >= 2);
        assert!(program
            .functions
            .iter()
            .flat_map(|f| &f.code)
            .any(|i| matches!(i.opcode, Opcode::GetProperty(_))));
        assert!(program
            .functions
            .iter()
            .flat_map(|f| &f.code)
            .any(|i| matches!(i.opcode, Opcode::GetIndex)));
        assert!(program
            .functions
            .iter()
            .flat_map(|f| &f.code)
            .any(|i| matches!(i.opcode, Opcode::SetIndex)));
        assert!(program
            .functions
            .iter()
            .flat_map(|f| &f.code)
            .any(|i| matches!(i.opcode, Opcode::Call(_))));
        assert!(program
            .functions
            .iter()
            .flat_map(|f| &f.code)
            .any(|i| matches!(i.opcode, Opcode::Jump(_))));
    }

    #[test]
    fn compiles_gradual_type_contract_metadata() {
        let source = "let value: number = 1; fn identity(item: number) -> number { return item; } print(identity(value));";
        let verified = compile_source("typed-compiler.hk", source).unwrap();
        let function = &verified.program().functions[1];
        assert!(function.typed);
        assert_eq!(function.return_contract, Some(Type::Number));
        assert_eq!(function.parameters[0].annotation, Some(TypeName::Number));
    }

    #[test]
    fn rejects_module_declarations_at_the_source_only_boundary() {
        let import_error =
            compile_source("import.hk", r#"import { value } from "./dep.hk";"#).unwrap_err();
        assert_eq!(import_error.code, "E4008");
        let export_program = compile_source("export.hk", "export let value = 1;").unwrap();
        assert_eq!(export_program.program().functions.len(), 1);
    }

    #[test]
    fn rejects_top_level_return_with_reference_diagnostic() {
        let error = compile_source("return.hk", "return 1;").unwrap_err();
        assert_eq!(error.code, "E3000");
        assert_eq!(error.message, "'return' is only valid inside a function.");
    }

    #[test]
    fn compiles_loop_scopes_and_array_operations() {
        let verified = compile_source(
            "loops.hk",
            "let a = [1, 2]; for (let i = 0; i < 2; i = i + 1) { a[i] = i; }",
        )
        .unwrap();
        assert!(verified.program().functions[0]
            .code
            .iter()
            .any(|instruction| matches!(instruction.opcode, Opcode::SetIndex)));
        assert!(verified.program().functions[0]
            .code
            .iter()
            .any(|instruction| matches!(instruction.opcode, Opcode::EnterScope)));
    }
}
