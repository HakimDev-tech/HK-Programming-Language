use super::ast::{Param, E, S};
use super::lexer::{Lit, K};
use super::types::Type;
use super::{HkError, Result};
use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    rc::{Rc, Weak},
};

#[derive(Clone)]
struct Fun {
    name: String,
    params: Vec<Param>,
    return_type: Option<Type>,
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
}
impl V {
    fn ty(&self) -> &'static str {
        match self {
            V::Num(_) => "Number",
            V::Str(_) => "String",
            V::Bool(_) => "Boolean",
            V::Null => "Null",
            V::Arr(_) => "Array",
            V::Fun(_) | V::Print => "Function",
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
                V::Fun(_) | V::Print => "function",
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

#[derive(Default)]
enum EnvError {
    Duplicate(String),
    Constant(String),
    Undefined(String),
    Contract(String),
}

struct Env {
    values: HashMap<String, V>,
    constants: HashSet<String>,
    contracts: HashMap<String, Type>,
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
        if self.values.contains_key(&n) {
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
    fn get(e: &Rc<RefCell<Self>>, n: &str) -> Option<V> {
        let mut current = Some(e.clone());
        while let Some(scope) = current {
            let (value, parent) = {
                let env = scope.borrow();
                (env.values.get(n).cloned(), env.parent.clone())
            };
            if value.is_some() {
                return value;
            }
            current = parent;
        }
        None
    }
    fn set(e: &Rc<RefCell<Self>>, n: &str, v: V) -> std::result::Result<(), EnvError> {
        let mut current = Some(e.clone());
        while let Some(scope) = current {
            let (exists, is_const, contract, parent) = {
                let env = scope.borrow();
                (
                    env.values.contains_key(n),
                    env.constants.contains(n),
                    env.contracts.get(n).copied(),
                    env.parent.clone(),
                )
            };
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
enum Flow {
    Next,
    Return(V, usize, usize),
}
struct Engine {
    env: Rc<RefCell<Env>>,
    output: Vec<String>,
    source: String,
    source_text: String,
    typed_mode: bool,
    call_depth: usize,
    eval_depth: usize,
    exec_depth: usize,
}
impl Engine {
    fn new(source: &str, source_text: &str, typed_mode: bool) -> Self {
        let e = Rc::new(RefCell::new(Env::default()));
        e.borrow_mut().values.insert("print".into(), V::Print);
        Self {
            env: e,
            output: vec![],
            source: source.into(),
            source_text: source_text.into(),
            typed_mode,
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
        let code = if self.typed_mode { "E3008" } else { "E3000" };
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
            S::TypedVar(name, annotation, initializer, mutable, line, column) => {
                let value = self.eval(initializer)?;
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
                    body: body.clone(),
                    env: Rc::downgrade(&self.env),
                }));
                self.env
                    .borrow_mut()
                    .define(name.clone(), function, false, None, "function declaration")
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
                    body: b.clone(),
                    env: Rc::downgrade(&self.env),
                }));
                self.env
                    .borrow_mut()
                    .define(n.clone(), f, false, None, "function declaration")
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
                let v = self.eval(c)?;
                let (line, col) = c.source_position();
                if self.boolean(v, line, col)? {
                    return self.exec(a);
                }
                if let Some(b) = b {
                    return self.exec(b);
                }
            }
            S::While(c, b) => loop {
                let v = self.eval(c)?;
                let (line, col) = c.source_position();
                if !self.boolean(v, line, col)? {
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
                            let v = self.eval(c)?;
                            let (line, col) = c.source_position();
                            if !self.boolean(v, line, col)? {
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
                return Ok(Flow::Return(
                    if let Some(e) = e {
                        self.eval(e)?
                    } else {
                        V::Null
                    },
                    *line,
                    *col,
                ))
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
        let result = self.eval_inner(e);
        self.eval_depth -= 1;
        result
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
                            Err(self.type_error("Unary '-' expects Number.", *l, *c))
                        }
                    }
                    K::Bang => {
                        if let V::Bool(b) = v {
                            Ok(V::Bool(!b))
                        } else {
                            Err(self.type_error("'!' expects Boolean.", *l, *c))
                        }
                    }
                    _ => unreachable!(),
                }
            }
            E::Binary(_, _, _, _, _) => self.eval_binary_chain(e),
            E::Call(callee, args, l, c) => {
                let f = self.eval(callee)?;
                let mut vs = vec![];
                for a in args {
                    vs.push(self.eval(a)?);
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
                        self.output.push(vs[0].show());
                        Ok(V::Null)
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
                        for (parameter, value) in fun.params.iter().zip(vs) {
                            call.borrow_mut()
                                .define(
                                    parameter.name.clone(),
                                    value,
                                    true,
                                    parameter.annotation.map(Type::from),
                                    "function parameter",
                                )
                                .map_err(|error| self.env_error(error, *l, *c))?;
                        }
                        let old = self.env.clone();
                        self.env = call;
                        self.call_depth += 1;
                        let r = self.exec_all(&fun.body);
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
                    v => Err(self.type_error(
                        format!("Cannot call a value of type {}.", v.ty()),
                        *l,
                        *c,
                    )),
                }
            }
            E::Index(a, i, l, c) => {
                let o = self.eval(a)?;
                let index_value = self.eval(i)?;
                let idx = self.index(index_value, *l, *c)?;
                if let V::Arr(items) = o {
                    items.borrow().get(idx).cloned().ok_or_else(|| {
                        self.error(format!("Array index {idx} is out of bounds."), *l, *c)
                    })
                } else {
                    Err(self.type_error("Indexing requires Array.", *l, *c))
                }
            }
            E::SetIndex(a, i, r, l, c) => {
                let o = self.eval(a)?;
                let index_value = self.eval(i)?;
                let idx = self.index(index_value, *l, *c)?;
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
                    Err(self.type_error("Indexed assignment requires Array.", *l, *c))
                }
            }
            E::Property(a, n, l, c) => {
                let o = self.eval(a)?;
                if n == "length" {
                    if let V::Arr(items) = o {
                        return Ok(V::Num(items.borrow().len() as f64));
                    }
                }
                Err(self.type_error(
                    format!("Property '{n}' is not available on {}.", o.ty()),
                    *l,
                    *c,
                ))
            }
        }
    }
    fn eval_binary_chain(&mut self, expression: &E) -> Result<V> {
        let mut operations: Vec<(K, &E, usize, usize)> = Vec::new();
        let mut cursor = expression;
        while let E::Binary(left, operator, right, line, column) = cursor {
            operations.push((*operator, right.as_ref(), *line, *column));
            cursor = left.as_ref();
        }

        let mut value = self.eval(cursor)?;
        for (operator, right, line, column) in operations.into_iter().rev() {
            match operator {
                K::And => {
                    if !self.boolean(value, line, column)? {
                        value = V::Bool(false);
                    } else {
                        let right_value = self.eval(right)?;
                        value = V::Bool(self.boolean(right_value, line, column)?);
                    }
                }
                K::Or => {
                    if self.boolean(value, line, column)? {
                        value = V::Bool(true);
                    } else {
                        let right_value = self.eval(right)?;
                        value = V::Bool(self.boolean(right_value, line, column)?);
                    }
                }
                _ => {
                    let right_value = self.eval(right)?;
                    value = self.binary(value, operator, right_value, line, column)?;
                }
            }
        }
        Ok(value)
    }

    fn binary(&self, a: V, op: K, b: V, l: usize, c: usize) -> Result<V> {
        match op {
            K::Plus => match (a, b) {
                (V::Num(x), V::Num(y)) => self.finite(x + y, l, c),
                (V::Str(x), V::Str(y)) => Ok(V::Str(x + &y)),
                (x, y) => Err(self.type_error(
                    format!(
                        "'+' expects two Numbers or two Strings, got {} and {}.",
                        x.ty(),
                        y.ty()
                    ),
                    l,
                    c,
                )),
            },
            K::Minus | K::Star | K::Slash | K::Percent => {
                let (x, y) = if let (V::Num(x), V::Num(y)) = (a, b) {
                    (x, y)
                } else {
                    return Err(self.type_error(
                        "Arithmetic operators require Number operands.",
                        l,
                        c,
                    ));
                };
                if (op == K::Slash || op == K::Percent) && y == 0.0 {
                    return Err(self.error("Division or modulo by zero.", l, c));
                }
                self.finite(
                    match op {
                        K::Minus => x - y,
                        K::Star => x * y,
                        K::Slash => x / y,
                        K::Percent => x % y,
                        _ => unreachable!(),
                    },
                    l,
                    c,
                )
            }
            K::Gt | K::GtEq | K::Lt | K::LtEq => {
                let (x, y) = if let (V::Num(x), V::Num(y)) = (a, b) {
                    (x, y)
                } else {
                    return Err(self.type_error(
                        "Ordering comparisons require Number operands.",
                        l,
                        c,
                    ));
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

pub(super) fn execute(
    source_name: &str,
    source_text: &str,
    statements: &[S],
    typed_mode: bool,
) -> Result<Vec<String>> {
    let mut engine = Engine::new(source_name, source_text, typed_mode);
    engine.run(statements)?;
    Ok(engine.output)
}
