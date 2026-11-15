use super::ast::{Param, E, S};
use super::lexer::{Lit, K};
use super::types::Type;
use super::{HkError, Result};
use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
    rc::{Rc, Weak},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MathFn {
    Abs,
    Ceil,
    Floor,
    Round,
    Sqrt,
    Min,
    Max,
}

#[derive(Clone)]
struct Fun {
    name: String,
    params: Vec<Param>,
    return_type: Option<Type>,
    typed: bool,
    body: Vec<S>,
    env: Weak<RefCell<Env>>,
}
#[derive(Clone)]
enum V {
    Num(f64),
    Str(String),
    Bool(bool),
    Null,
    Arr(Rc<RefCell<Vec<V>>>),
    Fun(Rc<Fun>),
    Print,
    Math(MathFn),
}
impl V {
    fn ty(&self) -> &'static str {
        match self {
            V::Num(_) => "Number",
            V::Str(_) => "String",
            V::Bool(_) => "Boolean",
            V::Null => "Null",
            V::Arr(_) => "Array",
            V::Fun(_) | V::Print | V::Math(_) => "Function",
        }
    }
    fn show(&self) -> String {
        self.show_with(&mut HashSet::new(), 0)
    }
    fn show_with(&self, seen: &mut HashSet<usize>, depth: usize) -> String {
        if depth >= 64 {
            return "<max-depth>".into();
        }
        match self {
            V::Num(n) if n.fract() == 0.0 => format!("{n:.0}"),
            V::Num(n) => n.to_string(),
            V::Str(s) => s.clone(),
            V::Bool(b) => b.to_string(),
            V::Null => "null".into(),
            V::Arr(a) => {
                let identity = Rc::as_ptr(a) as usize;
                if !seen.insert(identity) {
                    return "[<cycle>]".into();
                }
                let rendered = a
                    .borrow()
                    .iter()
                    .map(|value| value.show_with(seen, depth + 1))
                    .collect::<Vec<_>>()
                    .join(", ");
                seen.remove(&identity);
                format!("[{rendered}]")
            }
            V::Fun(f) => format!("<fn {}>", f.name),
            V::Print => "<native fn print>".into(),
            V::Math(function) => format!(
                "<native fn std:math::{}>",
                match function {
                    MathFn::Abs => "abs",
                    MathFn::Ceil => "ceil",
                    MathFn::Floor => "floor",
                    MathFn::Round => "round",
                    MathFn::Sqrt => "sqrt",
                    MathFn::Min => "min",
                    MathFn::Max => "max",
                }
            ),
        }
    }
    fn eq(&self, b: &V) -> bool {
        match (self, b) {
            (V::Num(a), V::Num(b)) => a == b,
            (V::Str(a), V::Str(b)) => a == b,
            (V::Bool(a), V::Bool(b)) => a == b,
            (V::Null, V::Null) => true,
            (V::Arr(a), V::Arr(b)) => Rc::ptr_eq(a, b),
            (V::Fun(a), V::Fun(b)) => Rc::ptr_eq(a, b),
            (V::Print, V::Print) => true,
            (V::Math(a), V::Math(b)) => a == b,
            _ => false,
        }
    }
}
fn value_matches(expected: Type, value: &V) -> bool {
    match expected {
        Type::Any => true,
        Type::Number => matches!(value, V::Num(_)),
        Type::String => matches!(value, V::Str(_)),
        Type::Bool => matches!(value, V::Bool(_)),
        Type::Null => matches!(value, V::Null),
        Type::Array => matches!(value, V::Arr(_)),
    }
}

fn ensure_contract(expected: Type, value: &V, boundary: &str) -> std::result::Result<(), String> {
    if value_matches(expected, value) {
        Ok(())
    } else {
        Err(format!(
            "Runtime type contract violation during {boundary}: expected {expected}, found {}.",
            match value {
                V::Num(_) => "number",
                V::Str(_) => "string",
                V::Bool(_) => "bool",
                V::Null => "null",
                V::Arr(_) => "array",
                V::Fun(_) | V::Print | V::Math(_) => "function",
            }
        ))
    }
}

fn contains_array_reference(value: &V, target: usize, visited: &mut HashSet<usize>) -> bool {
    let mut pending = vec![value.clone()];
    while let Some(current) = pending.pop() {
        if let V::Arr(array) = current {
            let identity = Rc::as_ptr(&array) as usize;
            if identity == target {
                return true;
            }
            if visited.insert(identity) {
                pending.extend(array.borrow().iter().cloned());
            }
        }
    }
    false
}

#[derive(Debug)]
enum EnvError {
    Duplicate(String),
    Constant(String),
    Undefined(String),
    Contract(String),
}

#[derive(Default)]
struct Env {
    values: HashMap<String, V>,
    constants: HashSet<String>,
    contracts: HashMap<String, Type>,
    function_names: HashSet<String>,
    aliases: HashMap<String, (Weak<RefCell<Env>>, String)>,
    parent: Option<Rc<RefCell<Env>>>,
}
impl Env {
    fn child(parent: Rc<RefCell<Env>>) -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(Self {
            parent: Some(parent),
            ..Self::default()
        }))
    }
    fn define(
        &mut self,
        n: String,
        v: V,
        m: bool,
        contract: Option<Type>,
        boundary: &str,
    ) -> std::result::Result<(), EnvError> {
        if self.values.contains_key(&n) || self.aliases.contains_key(&n) {
            return Err(EnvError::Duplicate(format!(
                "'{n}' is already declared in this scope."
            )));
        }
        if let Some(expected) = contract {
            ensure_contract(expected, &v, boundary).map_err(EnvError::Contract)?;
            self.contracts.insert(n.clone(), expected);
        }
        if !m {
            self.constants.insert(n.clone());
        }
        self.values.insert(n, v);
        Ok(())
    }
    fn define_alias(
        &mut self,
        name: String,
        target: &Rc<RefCell<Env>>,
        exported: String,
    ) -> std::result::Result<(), EnvError> {
        if self.values.contains_key(&name) || self.aliases.contains_key(&name) {
            return Err(EnvError::Duplicate(format!(
                "'{name}' is already declared in this scope."
            )));
        }
        self.aliases.insert(name, (Rc::downgrade(target), exported));
        Ok(())
    }
    fn define_function(&mut self, name: String, value: V) -> std::result::Result<(), EnvError> {
        self.define(name.clone(), value, false, None, "function declaration")?;
        self.function_names.insert(name);
        Ok(())
    }
    fn is_function_binding(e: &Rc<RefCell<Self>>, n: &str) -> bool {
        let mut current = Some(e.clone());
        while let Some(scope) = current {
            let (exists, is_function, alias, parent) = {
                let env = scope.borrow();
                (
                    env.values.contains_key(n),
                    env.function_names.contains(n),
                    env.aliases.get(n).cloned(),
                    env.parent.clone(),
                )
            };
            if exists {
                return is_function;
            }
            if let Some((target, exported)) = alias {
                return target
                    .upgrade()
                    .is_some_and(|target| Self::is_function_binding(&target, &exported));
            }
            current = parent;
        }
        false
    }
    fn has_contract(e: &Rc<RefCell<Self>>, n: &str) -> bool {
        let mut current = Some(e.clone());
        while let Some(scope) = current {
            let (contains, alias, parent) = {
                let env = scope.borrow();
                (
                    env.contracts.contains_key(n),
                    env.aliases.get(n).cloned(),
                    env.parent.clone(),
                )
            };
            if contains {
                return true;
            }
            if let Some((target, exported)) = alias {
                return target
                    .upgrade()
                    .is_some_and(|target| Self::has_contract(&target, &exported));
            }
            current = parent;
        }
        false
    }
    fn get(e: &Rc<RefCell<Self>>, n: &str) -> Option<V> {
        let mut current = Some(e.clone());
        while let Some(scope) = current {
            let (value, alias, parent) = {
                let env = scope.borrow();
                (
                    env.values.get(n).cloned(),
                    env.aliases.get(n).cloned(),
                    env.parent.clone(),
                )
            };
            if value.is_some() {
                return value;
            }
            if let Some((target, exported)) = alias {
                return target
                    .upgrade()
                    .and_then(|target| Self::get(&target, &exported));
            }
            current = parent;
        }
        None
    }
    fn set(e: &Rc<RefCell<Self>>, n: &str, v: V) -> std::result::Result<(), EnvError> {
        let mut current = Some(e.clone());
        while let Some(scope) = current {
            let (exists, is_const, contract, is_alias, parent) = {
                let env = scope.borrow();
                (
                    env.values.contains_key(n),
                    env.constants.contains(n),
                    env.contracts.get(n).copied(),
                    env.aliases.contains_key(n),
                    env.parent.clone(),
                )
            };
            if is_alias {
                return Err(EnvError::Constant(format!(
                    "Cannot reassign imported binding '{n}'."
                )));
            }
            if exists {
                if is_const {
                    return Err(EnvError::Constant(format!(
                        "Cannot reassign constant '{n}'."
                    )));
                }
                if let Some(expected) = contract {
                    ensure_contract(expected, &v, "assignment").map_err(EnvError::Contract)?;
                }
                scope.borrow_mut().values.insert(n.into(), v);
                return Ok(());
            }
            current = parent;
        }
        Err(EnvError::Undefined(format!("Undefined variable '{n}'.")))
    }
}
type SourcePosition = (usize, usize);

#[derive(Clone, Copy)]
struct BinaryPositions {
    operator: SourcePosition,
    left: SourcePosition,
    right: SourcePosition,
}

struct BinaryOperation<'a> {
    operator: K,
    right: &'a E,
    positions: BinaryPositions,
}

enum Flow {
    Next,
    Return(V, usize, usize),
}
struct Engine {
    env: Rc<RefCell<Env>>,
    output: Vec<String>,
    source: String,
    source_text: String,
    typed_context: bool,
    call_depth: usize,
    eval_depth: usize,
    exec_depth: usize,
}
impl Engine {
    fn new(source: &str, source_text: &str) -> Self {
        Self::new_with_env(source, source_text, Rc::new(RefCell::new(Env::default())))
    }
    fn new_with_env(source: &str, source_text: &str, e: Rc<RefCell<Env>>) -> Self {
        if !e.borrow().values.contains_key("print") {
            e.borrow_mut().values.insert("print".into(), V::Print);
        }
        Self {
            env: e,
            output: vec![],
            source: source.into(),
            source_text: source_text.into(),
            typed_context: false,
            call_depth: 0,
            eval_depth: 0,
            exec_depth: 0,
        }
    }
    fn error(&self, msg: impl Into<String>, line: usize, col: usize) -> HkError {
        HkError::new("E3000", msg, &self.source, line, col, &self.source_text)
    }
    fn contract_error(&self, msg: impl Into<String>, line: usize, col: usize) -> HkError {
        HkError::new("E3008", msg, &self.source, line, col, &self.source_text)
    }
    fn env_error(&self, error: EnvError, line: usize, column: usize) -> HkError {
        match error {
            EnvError::Contract(message) => self.contract_error(message, line, column),
            EnvError::Duplicate(message)
            | EnvError::Constant(message)
            | EnvError::Undefined(message) => self.error(message, line, column),
        }
    }
    fn type_error(&self, msg: impl Into<String>, line: usize, col: usize) -> HkError {
        let code = if self.typed_context { "E3008" } else { "E3000" };
        HkError::new(code, msg, &self.source, line, col, &self.source_text)
    }
    fn run(&mut self, ss: &[S]) -> Result<()> {
        for s in ss {
            if let Flow::Return(_, line, col) = self.exec(s)? {
                return Err(self.error("'return' is only valid inside a function.", line, col));
            }
        }
        Ok(())
    }
    fn exec_all(&mut self, ss: &[S]) -> Result<Flow> {
        for s in ss {
            match self.exec(s)? {
                Flow::Next => {}
                r @ Flow::Return(..) => return Ok(r),
            }
        }
        Ok(Flow::Next)
    }
    fn exec(&mut self, s: &S) -> Result<Flow> {
        if self.exec_depth >= 128 {
            let (line, column) = s.source_position();
            return Err(self.error(
                "Maximum statement execution depth (128) exceeded.",
                line,
                column,
            ));
        }
        self.exec_depth += 1;
        let result = self.exec_inner(s);
        self.exec_depth -= 1;
        result
    }
    fn exec_inner(&mut self, s: &S) -> Result<Flow> {
        match s {
            S::Import(..) => {}
            S::Export(statement) => return self.exec(statement),
            S::TypedVar(name, annotation, initializer, mutable, line, column) => {
                let previous_typed_context = self.typed_context;
                self.typed_context = true;
                let evaluated = self.eval(initializer);
                self.typed_context = previous_typed_context;
                let value = evaluated?;
                self.env
                    .borrow_mut()
                    .define(
                        name.clone(),
                        value,
                        *mutable,
                        Some(Type::from(*annotation)),
                        "variable initialization",
                    )
                    .map_err(|error| self.env_error(error, *line, *column))?;
            }
            S::TypedFn(name, parameters, return_type, body, line, column) => {
                let function = V::Fun(Rc::new(Fun {
                    name: name.clone(),
                    params: parameters.clone(),
                    return_type: return_type.map(Type::from),
                    typed: true,
                    body: body.clone(),
                    env: Rc::downgrade(&self.env),
                }));
                self.env
                    .borrow_mut()
                    .define_function(name.clone(), function)
                    .map_err(|error| self.env_error(error, *line, *column))?;
            }
            S::Var(n, e, m, l, c) => {
                let v = self.eval(e)?;
                self.env
                    .borrow_mut()
                    .define(n.clone(), v, *m, None, "variable initialization")
                    .map_err(|error| self.env_error(error, *l, *c))?;
            }
            S::Fn(n, p, b, l, c) => {
                let f = V::Fun(Rc::new(Fun {
                    name: n.clone(),
                    params: p
                        .iter()
                        .map(|name| Param {
                            name: name.clone(),
                            annotation: None,
                        })
                        .collect(),
                    return_type: None,
                    typed: false,
                    body: b.clone(),
                    env: Rc::downgrade(&self.env),
                }));
                self.env
                    .borrow_mut()
                    .define_function(n.clone(), f)
                    .map_err(|error| self.env_error(error, *l, *c))?;
            }
            S::Expr(e) => {
                self.eval(e)?;
            }
            S::Block(b) => {
                let old = self.env.clone();
                self.env = Env::child(old.clone());
                let r = self.exec_all(b);
                self.env = old;
                return r;
            }
            S::If(c, a, b) => {
                if self.eval_condition(c)? {
                    return self.exec(a);
                }
                if let Some(b) = b {
                    return self.exec(b);
                }
            }
            S::While(c, b) => loop {
                if !self.eval_condition(c)? {
                    break;
                }
                if let Flow::Return(v, line, col) = self.exec(b)? {
                    return Ok(Flow::Return(v, line, col));
                }
            },
            S::For(init, cond, inc, body) => {
                let old = self.env.clone();
                self.env = Env::child(old.clone());
                let r = (|| {
                    if let Some(i) = init {
                        self.exec(i)?;
                    }
                    loop {
                        if let Some(c) = cond {
                            if !self.eval_condition(c)? {
                                break;
                            }
                        }
                        if let Flow::Return(v, line, col) = self.exec(body)? {
                            return Ok(Flow::Return(v, line, col));
                        }
                        if let Some(i) = inc {
                            self.eval(i)?;
                        }
                    }
                    Ok(Flow::Next)
                })();
                self.env = old;
                return r;
            }
            S::Return(e, line, col) => {
                let (return_line, return_column) =
                    e.as_ref().map(E::source_position).unwrap_or((*line, *col));
                return Ok(Flow::Return(
                    if let Some(e) = e {
                        self.eval(e)?
                    } else {
                        V::Null
                    },
                    return_line,
                    return_column,
                ));
            }
        }
        Ok(Flow::Next)
    }
    fn eval(&mut self, e: &E) -> Result<V> {
        if self.eval_depth >= 128 {
            let (line, col) = e.source_position();
            return Err(self.error(
                "Maximum expression evaluation depth (128) exceeded.",
                line,
                col,
            ));
        }
        self.eval_depth += 1;
        let previous_typed_context = self.typed_context;
        self.typed_context |= self.expression_has_typed_provenance(e);
        let result = self.eval_inner(e);
        self.typed_context = previous_typed_context;
        self.eval_depth -= 1;
        result
    }
    fn expression_has_typed_provenance(&self, expression: &E) -> bool {
        match expression {
            E::Var(name, _, _) => {
                Env::has_contract(&self.env, name)
                    || matches!(Env::get(&self.env, name), Some(V::Fun(function)) if function.typed)
            }
            E::Assign(name, value, _, _) => {
                Env::has_contract(&self.env, name) || self.expression_has_typed_provenance(value)
            }
            E::Unary(_, operand, _, _) | E::Group(operand) => {
                self.expression_has_typed_provenance(operand)
            }
            E::Binary(left, _, right, _, _) => {
                self.expression_has_typed_provenance(left)
                    || self.expression_has_typed_provenance(right)
            }
            E::Call(_, arguments, _, _) => arguments
                .iter()
                .any(|argument| self.expression_has_typed_provenance(argument)),
            E::Array(elements, _, _) => elements
                .iter()
                .any(|element| self.expression_has_typed_provenance(element)),
            E::Index(receiver, index, _, _) => {
                self.expression_has_typed_provenance(receiver)
                    || self.expression_has_typed_provenance(index)
            }
            E::SetIndex(receiver, index, value, _, _) => {
                self.expression_has_typed_provenance(receiver)
                    || self.expression_has_typed_provenance(index)
                    || self.expression_has_typed_provenance(value)
            }
            E::Property(receiver, _, _, _) => self.expression_has_typed_provenance(receiver),
            E::Lit(_, _, _) | E::Bool(_, _, _) | E::Null(_, _) => false,
        }
    }
    fn eval_inner(&mut self, e: &E) -> Result<V> {
        let _source_position = e.source_position();
        match e {
            E::Lit(Lit::Number(n), _, _) => Ok(V::Num(*n)),
            E::Lit(Lit::String(s), _, _) => Ok(V::Str(s.clone())),
            E::Bool(b, _, _) => Ok(V::Bool(*b)),
            E::Null(_, _) => Ok(V::Null),
            E::Var(n, l, c) => Env::get(&self.env, n)
                .ok_or_else(|| self.error(format!("Undefined variable '{n}'."), *l, *c)),
            E::Assign(n, e, l, c) => {
                let v = self.eval(e)?;
                Env::set(&self.env, n, v.clone()).map_err(|error| self.env_error(error, *l, *c))?;
                Ok(v)
            }
            E::Group(e) => self.eval(e),
            E::Array(es, _, _) => {
                let mut a = vec![];
                for e in es {
                    a.push(self.eval(e)?);
                }
                Ok(V::Arr(Rc::new(RefCell::new(a))))
            }
            E::Unary(op, e, l, c) => {
                let v = self.eval(e)?;
                match op {
                    K::Minus => {
                        if let V::Num(n) = v {
                            self.finite(-n, *l, *c)
                        } else {
                            let (operand_line, operand_column) = e.source_position();
                            Err(self.type_error(
                                "Unary '-' expects Number.",
                                operand_line,
                                operand_column,
                            ))
                        }
                    }
                    K::Bang => {
                        if let V::Bool(b) = v {
                            Ok(V::Bool(!b))
                        } else {
                            let (operand_line, operand_column) = e.source_position();
                            Err(self.type_error(
                                "'!' expects Boolean.",
                                operand_line,
                                operand_column,
                            ))
                        }
                    }
                    _ => unreachable!(),
                }
            }
            E::Binary(_, _, _, _, _) => self.eval_binary_chain(e),
            E::Call(callee, args, l, c) => {
                let f = self.eval(callee)?;
                let direct_named_call = if let E::Var(name, _, _) = callee.as_ref() {
                    Env::is_function_binding(&self.env, name)
                } else {
                    false
                };
                let parameter_contracts = match &f {
                    V::Fun(function) if direct_named_call => Some(
                        function
                            .params
                            .iter()
                            .map(|parameter| parameter.annotation.map(Type::from))
                            .collect::<Vec<_>>(),
                    ),
                    _ => None,
                };
                let mut vs = Vec::with_capacity(args.len());
                for (index, argument) in args.iter().enumerate() {
                    let previous_typed_context = self.typed_context;
                    if parameter_contracts
                        .as_ref()
                        .and_then(|contracts| contracts.get(index))
                        .is_some_and(|contract| contract.is_some_and(|ty| ty != Type::Any))
                    {
                        self.typed_context = true;
                    }
                    let evaluated = self.eval(argument);
                    self.typed_context = previous_typed_context;
                    let value = evaluated?;
                    let (argument_line, argument_column) = argument.source_position();
                    vs.push((value, argument_line, argument_column));
                }
                match f {
                    V::Print => {
                        if vs.len() != 1 {
                            return Err(self.error(
                                format!("print() expects one argument, received {}.", vs.len()),
                                *l,
                                *c,
                            ));
                        }
                        self.output.push(vs[0].0.show());
                        Ok(V::Null)
                    }
                    V::Math(function) => {
                        let expected_arity = match function {
                            MathFn::Min | MathFn::Max => 2,
                            _ => 1,
                        };
                        if vs.len() != expected_arity {
                            return Err(self.error(format!("std:math function expects {expected_arity} argument(s), received {}.", vs.len()), *l, *c));
                        }
                        let mut numbers = Vec::with_capacity(vs.len());
                        for (value, line, column) in vs {
                            if let V::Num(number) = value {
                                if !number.is_finite() {
                                    return Err(self.error(
                                        "std:math arguments must be finite Numbers.",
                                        line,
                                        column,
                                    ));
                                }
                                numbers.push(number);
                            } else {
                                return Err(self.type_error(
                                    "std:math arguments must be Numbers.",
                                    line,
                                    column,
                                ));
                            }
                        }
                        let result = match function {
                            MathFn::Abs => numbers[0].abs(),
                            MathFn::Ceil => numbers[0].ceil(),
                            MathFn::Floor => numbers[0].floor(),
                            MathFn::Round => numbers[0].round(),
                            MathFn::Sqrt if numbers[0] < 0.0 => {
                                return Err(self.error(
                                    "std:math::sqrt requires a non-negative argument.",
                                    *l,
                                    *c,
                                ))
                            }
                            MathFn::Sqrt => numbers[0].sqrt(),
                            MathFn::Min => numbers[0].min(numbers[1]),
                            MathFn::Max => numbers[0].max(numbers[1]),
                        };
                        self.finite(result, *l, *c)
                    }
                    V::Fun(fun) => {
                        if vs.len() != fun.params.len() {
                            return Err(self.error(
                                format!(
                                    "Function '{}' expects {} arguments, received {}.",
                                    fun.name,
                                    fun.params.len(),
                                    vs.len()
                                ),
                                *l,
                                *c,
                            ));
                        }
                        if self.call_depth >= 64 {
                            return Err(self.error(
                                "Maximum function call depth (64) exceeded.",
                                *l,
                                *c,
                            ));
                        }
                        let parent = fun.env.upgrade().ok_or_else(|| {
                            self.error("Function environment is no longer available.", *l, *c)
                        })?;
                        let call = Env::child(parent);
                        for (parameter, (value, argument_line, argument_column)) in
                            fun.params.iter().zip(vs)
                        {
                            call.borrow_mut()
                                .define(
                                    parameter.name.clone(),
                                    value,
                                    true,
                                    parameter.annotation.map(Type::from),
                                    "function parameter",
                                )
                                .map_err(|error| {
                                    self.env_error(error, argument_line, argument_column)
                                })?;
                        }
                        let old = self.env.clone();
                        self.env = call;
                        self.call_depth += 1;
                        let previous_typed_context = self.typed_context;
                        self.typed_context = fun.typed;
                        let r = self.exec_all(&fun.body);
                        self.typed_context = previous_typed_context;
                        self.call_depth -= 1;
                        self.env = old;
                        let (value, return_line, return_column) = match r? {
                            Flow::Next => (V::Null, *l, *c),
                            Flow::Return(value, line, column) => (value, line, column),
                        };
                        if let Some(expected) = fun.return_type {
                            ensure_contract(expected, &value, "function return").map_err(
                                |message| self.contract_error(message, return_line, return_column),
                            )?;
                        }
                        Ok(value)
                    }
                    v => {
                        Err(self.error(format!("Cannot call a value of type {}.", v.ty()), *l, *c))
                    }
                }
            }
            E::Index(a, i, l, c) => {
                let o = self.eval(a)?;
                let index_value = self.eval(i)?;
                let (index_line, index_column) = i.source_position();
                let idx = self.index(index_value, index_line, index_column)?;
                if let V::Arr(items) = o {
                    items.borrow().get(idx).cloned().ok_or_else(|| {
                        self.error(format!("Array index {idx} is out of bounds."), *l, *c)
                    })
                } else {
                    let (receiver_line, receiver_column) = a.source_position();
                    Err(self.type_error("Indexing requires Array.", receiver_line, receiver_column))
                }
            }
            E::SetIndex(a, i, r, l, c) => {
                let o = self.eval(a)?;
                let index_value = self.eval(i)?;
                let (index_line, index_column) = i.source_position();
                let idx = self.index(index_value, index_line, index_column)?;
                let v = self.eval(r)?;
                if let V::Arr(items) = o {
                    if idx >= items.borrow().len() {
                        return Err(self.error(
                            format!("Array index {idx} is out of bounds."),
                            *l,
                            *c,
                        ));
                    }
                    let identity = Rc::as_ptr(&items) as usize;
                    if contains_array_reference(&v, identity, &mut HashSet::new()) {
                        return Err(self.error(
                            "Cyclic array references are not supported in HK 0.1.x.",
                            *l,
                            *c,
                        ));
                    }
                    let mut items = items.borrow_mut();
                    items[idx] = v.clone();
                    Ok(v)
                } else {
                    let (receiver_line, receiver_column) = a.source_position();
                    Err(self.type_error(
                        "Indexed assignment requires Array.",
                        receiver_line,
                        receiver_column,
                    ))
                }
            }
            E::Property(a, n, l, c) => {
                let value = self.eval(a)?;
                if n == "length" {
                    return match value {
                        V::Arr(items) => Ok(V::Num(items.borrow().len() as f64)),
                        other => {
                            let (receiver_line, receiver_column) = a.source_position();
                            Err(self.type_error(
                                format!(
                                    "Property 'length' requires Array, received {}.",
                                    other.ty()
                                ),
                                receiver_line,
                                receiver_column,
                            ))
                        }
                    };
                }
                Err(self.error(
                    format!("Property '{n}' is not available on {}.", value.ty()),
                    *l,
                    *c,
                ))
            }
        }
    }
    fn eval_binary_chain(&mut self, expression: &E) -> Result<V> {
        let mut operations = Vec::new();
        let mut cursor = expression;
        while let E::Binary(left, operator, right, line, column) = cursor {
            operations.push(BinaryOperation {
                operator: *operator,
                right: right.as_ref(),
                positions: BinaryPositions {
                    operator: (*line, *column),
                    left: left.source_position(),
                    right: right.source_position(),
                },
            });
            cursor = left.as_ref();
        }

        let mut value = self.eval(cursor)?;
        for operation in operations.into_iter().rev() {
            match operation.operator {
                K::And => {
                    if !self.boolean(
                        value,
                        operation.positions.left.0,
                        operation.positions.left.1,
                    )? {
                        value = V::Bool(false);
                    } else {
                        let right_value = self.eval(operation.right)?;
                        value = V::Bool(self.boolean(
                            right_value,
                            operation.positions.right.0,
                            operation.positions.right.1,
                        )?);
                    }
                }
                K::Or => {
                    if self.boolean(
                        value,
                        operation.positions.left.0,
                        operation.positions.left.1,
                    )? {
                        value = V::Bool(true);
                    } else {
                        let right_value = self.eval(operation.right)?;
                        value = V::Bool(self.boolean(
                            right_value,
                            operation.positions.right.0,
                            operation.positions.right.1,
                        )?);
                    }
                }
                _ => {
                    let right_value = self.eval(operation.right)?;
                    value =
                        self.binary(value, operation.operator, right_value, operation.positions)?;
                }
            }
        }
        Ok(value)
    }

    fn binary(&self, a: V, op: K, b: V, positions: BinaryPositions) -> Result<V> {
        match op {
            K::Plus => match (a, b) {
                (V::Num(x), V::Num(y)) => {
                    self.finite(x + y, positions.operator.0, positions.operator.1)
                }
                (V::Str(x), V::Str(y)) => Ok(V::Str(x + &y)),
                (x, y) => Err(self.error(
                    format!(
                        "'+' expects two Numbers or two Strings, got {} and {}.",
                        x.ty(),
                        y.ty()
                    ),
                    positions.operator.0,
                    positions.operator.1,
                )),
            },
            K::Minus | K::Star | K::Slash | K::Percent => {
                let (x, y) = match (a, b) {
                    (V::Num(x), V::Num(y)) => (x, y),
                    (left, _) if !matches!(&left, V::Num(_)) => {
                        return Err(self.type_error(
                            "Arithmetic operators require Number operands.",
                            positions.left.0,
                            positions.left.1,
                        ));
                    }
                    (_, _) => {
                        return Err(self.type_error(
                            "Arithmetic operators require Number operands.",
                            positions.right.0,
                            positions.right.1,
                        ));
                    }
                };
                if (op == K::Slash || op == K::Percent) && y == 0.0 {
                    return Err(self.error(
                        "Division or modulo by zero.",
                        positions.operator.0,
                        positions.operator.1,
                    ));
                }
                self.finite(
                    match op {
                        K::Minus => x - y,
                        K::Star => x * y,
                        K::Slash => x / y,
                        K::Percent => x % y,
                        _ => unreachable!(),
                    },
                    positions.operator.0,
                    positions.operator.1,
                )
            }
            K::Gt | K::GtEq | K::Lt | K::LtEq => {
                let (x, y) = match (a, b) {
                    (V::Num(x), V::Num(y)) => (x, y),
                    (left, _) if !matches!(&left, V::Num(_)) => {
                        return Err(self.type_error(
                            "Ordering comparisons require Number operands.",
                            positions.left.0,
                            positions.left.1,
                        ));
                    }
                    (_, _) => {
                        return Err(self.type_error(
                            "Ordering comparisons require Number operands.",
                            positions.right.0,
                            positions.right.1,
                        ));
                    }
                };
                Ok(V::Bool(match op {
                    K::Gt => x > y,
                    K::GtEq => x >= y,
                    K::Lt => x < y,
                    K::LtEq => x <= y,
                    _ => unreachable!(),
                }))
            }
            K::EqEq => Ok(V::Bool(a.eq(&b))),
            K::BangEq => Ok(V::Bool(!a.eq(&b))),
            _ => unreachable!(),
        }
    }
    fn finite(&self, n: f64, l: usize, c: usize) -> Result<V> {
        if n.is_finite() {
            Ok(V::Num(n))
        } else {
            Err(self.error("Numeric operation produced a non-finite result.", l, c))
        }
    }
    fn eval_condition(&mut self, expression: &E) -> Result<bool> {
        let previous_typed_context = self.typed_context;
        self.typed_context |= self.expression_has_typed_provenance(expression);
        let result = (|| {
            let value = self.eval(expression)?;
            let (line, column) = expression.source_position();
            self.boolean(value, line, column)
        })();
        self.typed_context = previous_typed_context;
        result
    }
    fn boolean(&self, v: V, l: usize, c: usize) -> Result<bool> {
        if let V::Bool(b) = v {
            Ok(b)
        } else {
            Err(self.type_error(
                format!("Condition must be Boolean, received {}.", v.ty()),
                l,
                c,
            ))
        }
    }
    fn index(&self, v: V, l: usize, c: usize) -> Result<usize> {
        if let V::Num(n) = v {
            if n >= 0.0 && n.fract() == 0.0 && n < (usize::MAX as f64) {
                Ok(n as usize)
            } else {
                Err(self.error("Array index must be a non-negative integer.", l, c))
            }
        } else {
            Err(self.type_error("Array index must be Number.", l, c))
        }
    }
}

struct LoadedModule {
    path: PathBuf,
    source: String,
    statements: Vec<S>,
    exports: HashSet<String>,
}

fn module_error(
    code: &'static str,
    message: impl Into<String>,
    source: &str,
    line: usize,
    column: usize,
    source_text: &str,
) -> HkError {
    HkError::new(code, message, source, line, column, source_text)
}

fn exported_name(statement: &S) -> Option<&str> {
    match statement {
        S::Export(inner) => match inner.as_ref() {
            S::Var(name, ..) | S::TypedVar(name, ..) | S::Fn(name, ..) | S::TypedFn(name, ..) => {
                Some(name)
            }
            _ => None,
        },
        _ => None,
    }
}

fn load_module_graph(
    requested: &Path,
    modules: &mut HashMap<PathBuf, LoadedModule>,
    order: &mut Vec<PathBuf>,
    visiting: &mut Vec<PathBuf>,
) -> Result<PathBuf> {
    if requested
        .extension()
        .and_then(|extension| extension.to_str())
        != Some("hk")
    {
        return Err(module_error(
            "E4002",
            "The entry file must use the .hk extension.",
            &requested.display().to_string(),
            1,
            1,
            "",
        ));
    }
    if visiting.len() >= 128 {
        return Err(module_error(
            "E4002",
            "Maximum module dependency depth (128) exceeded.",
            &requested.display().to_string(),
            1,
            1,
            "",
        ));
    }
    let canonical = fs::canonicalize(requested).map_err(|error| {
        module_error(
            "E4002",
            format!("Cannot resolve module '{}': {error}.", requested.display()),
            &requested.display().to_string(),
            1,
            1,
            "",
        )
    })?;
    if let Some(index) = visiting.iter().position(|path| path == &canonical) {
        let mut chain = visiting[index..]
            .iter()
            .map(|path| path.display().to_string())
            .collect::<Vec<_>>();
        chain.push(canonical.display().to_string());
        return Err(module_error(
            "E4005",
            format!("Cyclic module dependency: {}.", chain.join(" -> ")),
            &canonical.display().to_string(),
            1,
            1,
            "",
        ));
    }
    if modules.contains_key(&canonical) {
        return Ok(canonical);
    }
    if modules.len() >= 1024 {
        return Err(module_error(
            "E4002",
            "Module graph exceeds the limit of 1024 files.",
            &canonical.display().to_string(),
            1,
            1,
            "",
        ));
    }
    let source = fs::read_to_string(&canonical).map_err(|error| {
        module_error(
            "E4006",
            format!("Cannot read module '{}': {error}.", canonical.display()),
            &canonical.display().to_string(),
            1,
            1,
            "",
        )
    })?;
    let source_name = canonical.display().to_string();
    let tokens = super::lexer::Lexer::new(&source_name, &source).scan()?;
    let statements = super::parser::Parser::new(tokens, &source_name, &source).parse()?;

    let mut exports = HashSet::new();
    for statement in &statements {
        if let Some(name) = exported_name(statement) {
            if !exports.insert(name.to_owned()) {
                let (line, column) = statement.source_position();
                return Err(module_error(
                    "E4004",
                    format!("Duplicate export '{name}'."),
                    &source_name,
                    line,
                    column,
                    &source,
                ));
            }
        } else if matches!(statement, S::Export(_)) {
            let (line, column) = statement.source_position();
            return Err(module_error(
                "E4004",
                "Only top-level variables and named functions can be exported.",
                &source_name,
                line,
                column,
                &source,
            ));
        }
    }

    modules.insert(
        canonical.clone(),
        LoadedModule {
            path: canonical.clone(),
            source: source.clone(),
            statements: statements.clone(),
            exports,
        },
    );
    visiting.push(canonical.clone());

    for statement in &statements {
        if let S::Import(specifier, _, line, column) = statement {
            if specifier == "std:math" {
                continue;
            }
            let spec = Path::new(specifier);
            if spec.is_absolute()
                || spec.extension().and_then(|extension| extension.to_str()) != Some("hk")
                || specifier.starts_with("std:")
            {
                return Err(module_error(
                    "E4002",
                    format!("Unsupported module specifier '{specifier}'; use a relative .hk path or std:math."),
                    &source_name,
                    *line,
                    *column,
                    &source,
                ));
            }
            let target = canonical
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .join(spec);
            let target = fs::canonicalize(&target).map_err(|error| {
                module_error(
                    "E4002",
                    format!("Cannot resolve module '{specifier}': {error}."),
                    &source_name,
                    *line,
                    *column,
                    &source,
                )
            })?;
            if let Some(index) = visiting.iter().position(|path| path == &target) {
                let mut chain = visiting[index..]
                    .iter()
                    .map(|path| path.display().to_string())
                    .collect::<Vec<_>>();
                chain.push(target.display().to_string());
                return Err(module_error(
                    "E4005",
                    format!("Cyclic module dependency: {}.", chain.join(" -> ")),
                    &source_name,
                    *line,
                    *column,
                    &source,
                ));
            }
            load_module_graph(&target, modules, order, visiting)?;
        }
    }

    visiting.pop();
    order.push(canonical.clone());
    Ok(canonical)
}

pub(super) fn execute_file(entry: &Path) -> Result<Vec<String>> {
    let mut modules = HashMap::<PathBuf, LoadedModule>::new();
    let mut order = Vec::<PathBuf>::new();
    let mut visiting = Vec::<PathBuf>::new();
    let entry = load_module_graph(entry, &mut modules, &mut order, &mut visiting)?;

    // Validate the complete top-level module namespace before execution.
    for path in &order {
        let module = modules.get(path).expect("loaded module must exist");
        let mut local_names = HashSet::new();
        for statement in &module.statements {
            if let S::Import(_, names, _, _) = statement {
                for name in names {
                    if !local_names.insert(name.local.clone()) {
                        return Err(module_error(
                            "E4001",
                            format!("Duplicate module binding '{}'.", name.local),
                            &module.path.display().to_string(),
                            name.line,
                            name.column,
                            &module.source,
                        ));
                    }
                }
            } else {
                let declaration = match statement {
                    S::Export(inner) => inner.as_ref(),
                    other => other,
                };
                let declared = match declaration {
                    S::Var(name, ..)
                    | S::TypedVar(name, ..)
                    | S::Fn(name, ..)
                    | S::TypedFn(name, ..) => Some((name.as_str(), declaration.source_position())),
                    _ => None,
                };
                if let Some((name, (line, column))) = declared {
                    if !local_names.insert(name.to_owned()) {
                        return Err(module_error(
                            "E4001",
                            format!("Duplicate module binding '{name}'."),
                            &module.path.display().to_string(),
                            line,
                            column,
                            &module.source,
                        ));
                    }
                }
            }
        }
    }

    // Validate every import link before any module body executes.
    for path in &order {
        let module = modules.get(path).expect("loaded module must exist");
        let source_name = module.path.display().to_string();
        for statement in &module.statements {
            if let S::Import(specifier, names, line, column) = statement {
                let available: HashSet<String> = if specifier == "std:math" {
                    ["abs", "ceil", "floor", "round", "sqrt", "min", "max"]
                        .into_iter()
                        .map(str::to_owned)
                        .collect()
                } else {
                    let spec = Path::new(specifier);
                    let target_path = fs::canonicalize(
                        module
                            .path
                            .parent()
                            .unwrap_or_else(|| Path::new("."))
                            .join(spec),
                    )
                    .map_err(|error| {
                        module_error(
                            "E4002",
                            format!("Cannot resolve module '{specifier}': {error}."),
                            &source_name,
                            *line,
                            *column,
                            &module.source,
                        )
                    })?;
                    modules
                        .get(&target_path)
                        .map(|target| target.exports.clone())
                        .unwrap_or_default()
                };
                for name in names {
                    if !available.contains(&name.imported) {
                        return Err(module_error(
                            "E4003",
                            format!("Module '{specifier}' does not export '{}'.", name.imported),
                            &source_name,
                            name.line,
                            name.column,
                            &module.source,
                        ));
                    }
                }
            }
        }
    }

    // Type-check the complete graph before executing any top-level statement.
    for path in &order {
        let module = modules.get(path).expect("loaded module must exist");
        if crate::has_type_annotations(&module.statements) {
            super::type_checker::TypeChecker::new(
                &module.path.display().to_string(),
                &module.source,
            )
            .check(&module.statements)?;
        }
    }

    // Allocate every environment before linking imports. Strong references in this
    // map keep defining module environments alive for imported functions.
    let mut environments = HashMap::<PathBuf, Rc<RefCell<Env>>>::new();
    for path in &order {
        environments.insert(path.clone(), Rc::new(RefCell::new(Env::default())));
    }
    let std_math = Rc::new(RefCell::new(Env::default()));
    for (name, function) in [
        ("abs", MathFn::Abs),
        ("ceil", MathFn::Ceil),
        ("floor", MathFn::Floor),
        ("round", MathFn::Round),
        ("sqrt", MathFn::Sqrt),
        ("min", MathFn::Min),
        ("max", MathFn::Max),
    ] {
        std_math
            .borrow_mut()
            .define_function(name.to_owned(), V::Math(function))
            .map_err(|error| {
                module_error(
                    "E4003",
                    format!("Cannot initialize std:math: {error:?}"),
                    "std:math",
                    1,
                    1,
                    "",
                )
            })?;
    }

    for path in &order {
        let module = modules.get(path).expect("loaded module must exist");
        let environment = environments
            .get(path)
            .expect("module environment must exist");
        for statement in &module.statements {
            if let S::Import(specifier, names, _, _) = statement {
                let target = if specifier == "std:math" {
                    std_math.clone()
                } else {
                    let target_path = fs::canonicalize(
                        module
                            .path
                            .parent()
                            .unwrap_or_else(|| Path::new("."))
                            .join(specifier),
                    )
                    .map_err(|error| {
                        module_error(
                            "E4002",
                            format!("Cannot resolve module '{specifier}': {error}."),
                            &module.path.display().to_string(),
                            1,
                            1,
                            &module.source,
                        )
                    })?;
                    environments
                        .get(&target_path)
                        .expect("linked module environment must exist")
                        .clone()
                };
                for name in names {
                    environment
                        .borrow_mut()
                        .define_alias(name.local.clone(), &target, name.imported.clone())
                        .map_err(|error| {
                            module_error(
                                "E4003",
                                format!("{error:?}"),
                                &module.path.display().to_string(),
                                name.line,
                                name.column,
                                &module.source,
                            )
                        })?;
                }
            }
        }
    }

    let mut output = Vec::new();
    for path in &order {
        let module = modules.get(path).expect("loaded module must exist");
        let environment = environments
            .get(path)
            .expect("module environment must exist")
            .clone();
        let mut engine = Engine::new_with_env(
            &module.path.display().to_string(),
            &module.source,
            environment,
        );
        engine.run(&module.statements)?;
        output.extend(engine.output);
    }
    let _entry_module = environments
        .get(&entry)
        .expect("entry module environment must exist");
    Ok(output)
}

pub(super) fn execute(
    source_name: &str,
    source_text: &str,
    statements: &[S],
) -> Result<Vec<String>> {
    let mut engine = Engine::new(source_name, source_text);
    engine.run(statements)?;
    Ok(engine.output)
}

#[cfg(test)]
mod contract_tests {
    use super::*;

    #[test]
    fn failed_contract_assignment_preserves_the_previous_binding_value() {
        let environment = Rc::new(RefCell::new(Env::default()));
        environment
            .borrow_mut()
            .define(
                "value".into(),
                V::Num(1.0),
                true,
                Some(Type::Number),
                "test initialization",
            )
            .unwrap();

        let error = Env::set(&environment, "value", V::Str("invalid".into())).unwrap_err();
        assert!(matches!(error, EnvError::Contract(_)));
        assert!(matches!(Env::get(&environment, "value"), Some(V::Num(value)) if value == 1.0));
    }

    #[test]
    fn failed_parameter_contract_does_not_enter_body_or_repeat_argument_evaluation() {
        let source = r#"
            let counter = 0;
            let values = ["first", "second"];
            fn take(value: number) -> number {
                print("entered");
                return value;
            }
            let operation = take;
            operation(values[counter = counter + 1]);
        "#;
        let tokens = crate::lexer::Lexer::new("contract-test.hk", source)
            .scan()
            .unwrap();
        let statements = crate::parser::Parser::new(tokens, "contract-test.hk", source)
            .parse()
            .unwrap();
        crate::type_checker::TypeChecker::new("contract-test.hk", source)
            .check(&statements)
            .unwrap();

        let mut engine = Engine::new("contract-test.hk", source);
        let error = engine.run(&statements).unwrap_err();

        assert_eq!(error.code, "E3008");
        assert!(engine.output.is_empty(), "function body must not run");
        assert!(matches!(
            Env::get(&engine.env, "counter"),
            Some(V::Num(value)) if value == 1.0
        ));
    }

    #[test]
    fn shadowed_bindings_keep_independent_runtime_contracts() {
        let parent = Rc::new(RefCell::new(Env::default()));
        parent
            .borrow_mut()
            .define(
                "value".into(),
                V::Num(1.0),
                true,
                Some(Type::Number),
                "test initialization",
            )
            .unwrap();

        let child = Env::child(parent.clone());
        child
            .borrow_mut()
            .define(
                "value".into(),
                V::Str("child".into()),
                true,
                Some(Type::String),
                "test initialization",
            )
            .unwrap();

        assert!(Env::set(&child, "value", V::Str("updated".into())).is_ok());
        assert!(matches!(Env::get(&parent, "value"), Some(V::Num(1.0))));
        assert!(matches!(
            Env::get(&child, "value"),
            Some(V::Str(value)) if value == "updated"
        ));
    }
}
