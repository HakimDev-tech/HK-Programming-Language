use std::collections::HashMap;

use crate::ast::{E, S};
use crate::lexer::{Lit, K};
use crate::{HkError, Result};

use super::types::Type;

#[derive(Clone, Copy)]
struct Binding {
    ty: Type,
    contract: Option<Type>,
    mutable: bool,
}
#[derive(Clone)]
struct FunctionSignature {
    parameters: Vec<Type>,
    return_type: Option<Type>,
    typed: bool,
}

pub(crate) struct TypeChecker<'a> {
    source_name: &'a str,
    source: &'a str,
    scopes: Vec<HashMap<String, Binding>>,
    functions: Vec<HashMap<String, FunctionSignature>>,
    return_type: Option<Type>,
    strict: bool,
    strict_context: bool,
}

impl<'a> TypeChecker<'a> {
    pub(crate) fn new(source_name: &'a str, source: &'a str) -> Self {
        Self {
            source_name,
            source,
            scopes: vec![HashMap::new()],
            functions: vec![HashMap::new()],
            return_type: None,
            strict: false,
            strict_context: false,
        }
    }

    pub(crate) fn check(&mut self, statements: &[S]) -> Result<()> {
        for statement in statements {
            self.check_statement(statement)?;
        }
        Ok(())
    }

    fn error(
        &self,
        code: &'static str,
        message: impl Into<String>,
        line: usize,
        column: usize,
    ) -> HkError {
        HkError::new(code, message, self.source_name, line, column, self.source)
    }

    fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
        self.functions.push(HashMap::new());
    }

    fn pop_scope(&mut self) {
        self.scopes.pop();
        self.functions.pop();
    }

    fn define_binding(&mut self, name: &str, binding: Binding) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.entry(name.to_owned()).or_insert(binding);
        }
    }

    fn binding(&self, name: &str) -> Option<Binding> {
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| scope.get(name).copied())
    }

    fn function(&self, name: &str) -> Option<FunctionSignature> {
        for (bindings, functions) in self.scopes.iter().zip(self.functions.iter()).rev() {
            if let Some(signature) = functions.get(name) {
                return Some(signature.clone());
            }
            if bindings.contains_key(name) {
                return None;
            }
        }
        None
    }

    fn check_statement(&mut self, statement: &S) -> Result<()> {
        match statement {
            S::Var(name, initializer, mutable, _, _) => {
                self.check_expression(initializer)?;
                self.define_binding(
                    name,
                    Binding {
                        ty: Type::Any,
                        contract: None,
                        mutable: *mutable,
                    },
                );
            }
            S::TypedVar(name, annotation, initializer, mutable, line, column) => {
                let expected = Type::from(*annotation);
                let actual = self.check_expression_with_strict(initializer, true)?;
                self.require(
                    expected,
                    actual,
                    "E3001",
                    "variable initializer",
                    *line,
                    *column,
                )?;
                self.define_binding(
                    name,
                    Binding {
                        ty: expected,
                        contract: Some(expected),
                        mutable: *mutable,
                    },
                );
            }
            S::Fn(name, parameters, body, _, _) => {
                let signature = FunctionSignature {
                    parameters: vec![Type::Any; parameters.len()],
                    return_type: None,
                    typed: false,
                };
                self.define_function(name, signature);
                self.define_binding(
                    name,
                    Binding {
                        ty: Type::Any,
                        contract: None,
                        mutable: false,
                    },
                );
                self.check_function_body(
                    parameters
                        .iter()
                        .map(|name| (name.as_str(), Type::Any, None))
                        .collect(),
                    body,
                    None,
                    false,
                )?;
            }
            S::TypedFn(name, parameters, return_type, body, _, _) => {
                let parameter_types = parameters
                    .iter()
                    .map(|parameter| parameter.annotation.map(Type::from).unwrap_or(Type::Any))
                    .collect::<Vec<_>>();
                let signature = FunctionSignature {
                    parameters: parameter_types.clone(),
                    return_type: return_type.map(Type::from),
                    typed: true,
                };
                self.define_function(name, signature);
                self.define_binding(
                    name,
                    Binding {
                        ty: Type::Any,
                        contract: None,
                        mutable: false,
                    },
                );
                let parameter_bindings = parameters
                    .iter()
                    .zip(parameter_types.iter())
                    .map(|(parameter, ty)| {
                        (
                            parameter.name.as_str(),
                            *ty,
                            parameter.annotation.map(Type::from),
                        )
                    })
                    .collect();
                self.check_function_body(
                    parameter_bindings,
                    body,
                    return_type.map(Type::from),
                    true,
                )?;
                if let Some(ty) = return_type.map(Type::from) {
                    if ty != Type::Any && ty != Type::Null && !guarantees_return(body) {
                        let (line, column) = statement.source_position();
                        return Err(self.error(
                            "E3009",
                            format!("Function '{name}' may reach its end without returning {ty}."),
                            line,
                            column,
                        ));
                    }
                }
            }
            S::Expr(expression) => {
                self.check_expression(expression)?;
            }
            S::Block(statements) => {
                self.push_scope();
                let result = self.check(statements);
                self.pop_scope();
                result?;
            }
            S::If(condition, then_branch, else_branch) => {
                let strict_condition = self.strict || self.expression_is_typed(condition);
                let actual = self.check_expression_with_strict(condition, strict_condition)?;
                if strict_condition {
                    let (line, column) = condition.source_position();
                    self.require(Type::Bool, actual, "E3006", "if condition", line, column)?;
                }

                // Declarations are only legal in explicit blocks. Those blocks
                // create their own lexical scopes, so the control-flow statement
                // itself must not introduce an additional checker scope.
                self.check_statement(then_branch)?;
                if let Some(branch) = else_branch {
                    self.check_statement(branch)?;
                }
            }
            S::While(condition, body) => {
                let strict_condition = self.strict || self.expression_is_typed(condition);
                let actual = self.check_expression_with_strict(condition, strict_condition)?;
                if strict_condition {
                    let (line, column) = condition.source_position();
                    self.require(Type::Bool, actual, "E3006", "while condition", line, column)?;
                }

                // The body may execute zero times. Any declarations are contained
                // in an explicit block, whose normal scope ends after the body.
                self.check_statement(body)?;
            }
            S::For(initializer, condition, increment, body) => {
                self.push_scope();
                let result = (|| {
                    if let Some(initializer) = initializer {
                        self.check_statement(initializer)?;
                    }
                    if let Some(condition) = condition {
                        let strict_condition = self.strict || self.expression_is_typed(condition);
                        let actual =
                            self.check_expression_with_strict(condition, strict_condition)?;
                        if strict_condition {
                            let (line, column) = condition.source_position();
                            self.require(
                                Type::Bool,
                                actual,
                                "E3006",
                                "for condition",
                                line,
                                column,
                            )?;
                        }
                    }
                    // Runtime executes the body before the increment. Analyze in
                    // that order; explicit block declarations leave scope normally.
                    self.check_statement(body)?;
                    if let Some(increment) = increment {
                        self.check_expression(increment)?;
                    }
                    Ok(())
                })();
                self.pop_scope();
                result?;
            }
            S::Return(expression, line, column) => {
                let actual = if let Some(expression) = expression {
                    self.check_expression_with_strict(
                        expression,
                        self.return_type.is_some() || self.strict,
                    )?
                } else {
                    Type::Null
                };
                if let Some(expected) = self.return_type {
                    self.require(expected, actual, "E3004", "function return", *line, *column)?;
                }
            }
        }
        Ok(())
    }

    fn define_function(&mut self, name: &str, signature: FunctionSignature) {
        if let Some(scope) = self.functions.last_mut() {
            scope.entry(name.to_owned()).or_insert(signature);
        }
    }

    fn check_function_body(
        &mut self,
        parameters: Vec<(&str, Type, Option<Type>)>,
        body: &[S],
        return_type: Option<Type>,
        strict: bool,
    ) -> Result<()> {
        self.push_scope();
        let previous_return_type = self.return_type;
        let previous_strict = self.strict;
        let previous_strict_context = self.strict_context;
        self.return_type = return_type;
        self.strict = strict;
        self.strict_context = strict;
        for (name, ty, contract) in parameters {
            self.define_binding(
                name,
                Binding {
                    ty,
                    contract,
                    mutable: true,
                },
            );
        }
        let result = self.check(body);
        self.return_type = previous_return_type;
        self.strict = previous_strict;
        self.strict_context = previous_strict_context;
        self.pop_scope();
        result
    }

    fn require(
        &self,
        expected: Type,
        actual: Type,
        code: &'static str,
        context: &str,
        line: usize,
        column: usize,
    ) -> Result<()> {
        if expected.accepts(actual) {
            Ok(())
        } else {
            Err(self.error(
                code,
                format!("Incompatible {context}: expected {expected}, found {actual}."),
                line,
                column,
            ))
        }
    }

    fn check_expression_with_strict(&mut self, expression: &E, strict: bool) -> Result<Type> {
        let previous = self.strict;
        let previous_context = self.strict_context;
        self.strict = strict;
        self.strict_context = strict;
        let result = self.check_expression(expression);
        self.strict = previous;
        self.strict_context = previous_context;
        result
    }

    fn expression_is_typed(&self, expression: &E) -> bool {
        match expression {
            E::Var(name, _, _) => self
                .binding(name)
                .is_some_and(|binding| binding.contract.is_some()),
            E::Assign(name, value, _, _) => {
                self.binding(name)
                    .is_some_and(|binding| binding.contract.is_some())
                    || self.expression_is_typed(value)
            }
            E::Unary(_, operand, _, _) | E::Group(operand) => self.expression_is_typed(operand),
            E::Binary(left, _, right, _, _) => {
                self.expression_is_typed(left) || self.expression_is_typed(right)
            }
            E::Call(callee, arguments, _, _) => {
                let typed_direct_call = if let E::Var(name, _, _) = callee.as_ref() {
                    self.function(name)
                        .and_then(|signature| signature.return_type)
                        .is_some_and(|return_type| return_type != Type::Any)
                } else {
                    false
                };
                typed_direct_call
                    || self.expression_is_typed(callee)
                    || arguments
                        .iter()
                        .any(|argument| self.expression_is_typed(argument))
            }
            E::Array(elements, _, _) => elements
                .iter()
                .any(|element| self.expression_is_typed(element)),
            E::Index(receiver, index, _, _) => {
                self.expression_is_typed(receiver) || self.expression_is_typed(index)
            }
            E::SetIndex(receiver, index, value, _, _) => {
                self.expression_is_typed(receiver)
                    || self.expression_is_typed(index)
                    || self.expression_is_typed(value)
            }
            E::Property(receiver, _, _, _) => self.expression_is_typed(receiver),
            E::Lit(_, _, _) | E::Bool(_, _, _) | E::Null(_, _) => false,
        }
    }

    fn check_expression(&mut self, expression: &E) -> Result<Type> {
        let previous = self.strict;
        self.strict = self.strict || self.strict_context || self.expression_is_typed(expression);
        let result = self.check_expression_inner(expression);
        self.strict = previous;
        result
    }

    fn check_expression_inner(&mut self, expression: &E) -> Result<Type> {
        match expression {
            E::Lit(Lit::Number(_), _, _) => Ok(Type::Number),
            E::Lit(Lit::String(_), _, _) => Ok(Type::String),
            E::Bool(_, _, _) => Ok(Type::Bool),
            E::Null(_, _) => Ok(Type::Null),
            E::Var(name, _, _) => Ok(self
                .binding(name)
                .map(|binding| binding.ty)
                .unwrap_or(Type::Any)),
            E::Assign(name, value, line, column) => {
                let contract = self.binding(name).and_then(|binding| binding.contract);
                let actual =
                    self.check_expression_with_strict(value, contract.is_some() || self.strict)?;
                if let Some(binding) = self.binding(name) {
                    if !binding.mutable {
                        return Err(self.error(
                            "E3002",
                            format!("Cannot assign to constant '{name}'."),
                            *line,
                            *column,
                        ));
                    }
                    if let Some(contract) = binding.contract {
                        self.require(contract, actual, "E3002", "assignment", *line, *column)?;
                        Ok(contract)
                    } else {
                        Ok(actual)
                    }
                } else {
                    Ok(actual)
                }
            }
            E::Unary(operator, operand, line, column) => {
                let actual = self.check_expression(operand)?;
                let expected = if *operator == K::Bang {
                    Type::Bool
                } else {
                    Type::Number
                };
                if self.strict {
                    self.require(expected, actual, "E3005", "unary operand", *line, *column)?;
                }
                Ok(expected)
            }
            E::Binary(left, operator, right, line, column) => {
                let left_type = self.check_expression(left)?;
                let right_type = self.check_expression(right)?;
                match operator {
                    K::Plus => {
                        if left_type == Type::Any || right_type == Type::Any {
                            Ok(Type::Any)
                        } else if left_type == Type::Number && right_type == Type::Number {
                            Ok(Type::Number)
                        } else if left_type == Type::String && right_type == Type::String {
                            Ok(Type::String)
                        } else if self.strict {
                            Err(self.error(
                                "E3005",
                                format!("'+' cannot combine {left_type} and {right_type}."),
                                *line,
                                *column,
                            ))
                        } else {
                            Ok(Type::Any)
                        }
                    }
                    K::Minus | K::Star | K::Slash | K::Percent => {
                        if self.strict {
                            self.require(
                                Type::Number,
                                left_type,
                                "E3005",
                                "left arithmetic operand",
                                *line,
                                *column,
                            )?;
                            self.require(
                                Type::Number,
                                right_type,
                                "E3005",
                                "right arithmetic operand",
                                *line,
                                *column,
                            )?;
                        }
                        Ok(Type::Number)
                    }
                    K::Gt | K::GtEq | K::Lt | K::LtEq => {
                        if self.strict {
                            self.require(
                                Type::Number,
                                left_type,
                                "E3005",
                                "left comparison operand",
                                *line,
                                *column,
                            )?;
                            self.require(
                                Type::Number,
                                right_type,
                                "E3005",
                                "right comparison operand",
                                *line,
                                *column,
                            )?;
                        }
                        Ok(Type::Bool)
                    }
                    K::And | K::Or => {
                        if self.strict {
                            self.require(
                                Type::Bool,
                                left_type,
                                "E3005",
                                "left logical operand",
                                *line,
                                *column,
                            )?;
                            self.require(
                                Type::Bool,
                                right_type,
                                "E3005",
                                "right logical operand",
                                *line,
                                *column,
                            )?;
                        }
                        Ok(Type::Bool)
                    }
                    K::EqEq | K::BangEq => Ok(Type::Bool),
                    _ => Ok(Type::Any),
                }
            }
            E::Group(inner) => self.check_expression(inner),
            E::Call(callee, arguments, line, column) => {
                if let E::Var(name, _, _) = callee.as_ref() {
                    if let Some(signature) = self.function(name) {
                        if !signature.typed {
                            for argument in arguments {
                                self.check_expression(argument)?;
                            }
                            return Ok(Type::Any);
                        }
                        if signature.parameters.len() != arguments.len() {
                            return Err(self.error(
                                "E3003",
                                format!(
                                    "Function '{name}' expects {} arguments, received {}.",
                                    signature.parameters.len(),
                                    arguments.len()
                                ),
                                *line,
                                *column,
                            ));
                        }
                        for (argument, expected) in arguments.iter().zip(signature.parameters) {
                            let actual = self.check_expression_with_strict(argument, expected != Type::Any || self.strict_context)?;
                            let (arg_line, arg_column) = argument.source_position();
                            self.require(
                                expected,
                                actual,
                                "E3003",
                                "function argument",
                                arg_line,
                                arg_column,
                            )?;
                        }
                        return Ok(signature.return_type.unwrap_or(Type::Any));
                    }
                }
                let callee_type = self.check_expression(callee)?;
                let typed_binding_callee = if let E::Var(name, _, _) = callee.as_ref() {
                    self.binding(name)
                        .is_some_and(|binding| binding.contract.is_some())
                } else {
                    false
                };
                if callee_type != Type::Any && (self.strict || typed_binding_callee) {
                    let (callee_line, callee_column) = callee.source_position();
                    return Err(self.error(
                        "E3010",
                        format!("Cannot call a value of type {callee_type}; expected a function."),
                        callee_line,
                        callee_column,
                    ));
                }
                for argument in arguments {
                    self.check_expression(argument)?;
                }
                Ok(Type::Any)
            }
            E::Array(elements, _, _) => {
                for element in elements {
                    self.check_expression(element)?;
                }
                Ok(Type::Array)
            }
            E::Index(receiver, index, line, column) => {
                let receiver_type = self.check_expression(receiver)?;
                let index_type = self.check_expression(index)?;
                if self.strict {
                    self.require(
                        Type::Array,
                        receiver_type,
                        "E3005",
                        "array receiver",
                        *line,
                        *column,
                    )?;
                    self.require(
                        Type::Number,
                        index_type,
                        "E3005",
                        "array index",
                        *line,
                        *column,
                    )?;
                }
                Ok(Type::Any)
            }
            E::SetIndex(receiver, index, value, line, column) => {
                let receiver_type = self.check_expression(receiver)?;
                let index_type = self.check_expression(index)?;
                let value_type = self.check_expression(value)?;
                if self.strict {
                    self.require(
                        Type::Array,
                        receiver_type,
                        "E3005",
                        "array receiver",
                        *line,
                        *column,
                    )?;
                    self.require(
                        Type::Number,
                        index_type,
                        "E3005",
                        "array index",
                        *line,
                        *column,
                    )?;
                }
                Ok(value_type)
            }
            E::Property(receiver, property, line, column) => {
                let receiver_type = self.check_expression(receiver)?;
                if property == "length" {
                    if self.strict {
                        self.require(
                            Type::Array,
                            receiver_type,
                            "E3005",
                            "property receiver",
                            *line,
                            *column,
                        )?;
                    }
                    Ok(Type::Number)
                } else {
                    Ok(Type::Any)
                }
            }
        }
    }
}

fn guarantees_return(statements: &[S]) -> bool {
    for statement in statements {
        if statement_guarantees_return(statement) {
            return true;
        }
    }
    false
}

fn statement_guarantees_return(statement: &S) -> bool {
    match statement {
        S::Return(_, _, _) => true,
        S::Block(statements) => guarantees_return(statements),
        S::If(_, then_branch, Some(else_branch)) => {
            statement_guarantees_return(then_branch) && statement_guarantees_return(else_branch)
        }
        _ => false,
    }
}
