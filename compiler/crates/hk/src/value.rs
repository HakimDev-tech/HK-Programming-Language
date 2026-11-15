//! Shared runtime value representation and value-level semantics.
//! Both the tree-walking reference engine and the bytecode VM use this module.
use super::ast::{Param, E, S};
use super::runtime::Env;
use super::types::Type;
use std::{
    cell::RefCell,
    collections::HashSet,
    rc::{Rc, Weak},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum MathFn {
    Abs,
    Ceil,
    Floor,
    Round,
    Sqrt,
    Min,
    Max,
}

#[derive(Clone)]
pub(super) struct Fun {
    pub(super) name: String,
    pub(super) params: Vec<Param>,
    pub(super) return_type: Option<Type>,
    pub(super) typed: bool,
    pub(super) body: Vec<S>,
    pub(super) env: Weak<RefCell<Env>>,
    pub(super) source: Rc<String>,
    pub(super) source_text: Rc<String>,
    pub(super) line: usize,
    pub(super) column: usize,
}
#[derive(Clone)]
pub(super) enum V {
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
    pub(super) fn ty(&self) -> &'static str {
        match self {
            V::Num(_) => "Number",
            V::Str(_) => "String",
            V::Bool(_) => "Boolean",
            V::Null => "Null",
            V::Arr(_) => "Array",
            V::Fun(_) | V::Print | V::Math(_) => "Function",
        }
    }
    pub(super) fn show(&self) -> String {
        self.show_with(&mut HashSet::new(), 0)
    }
    pub(super) fn show_with(&self, seen: &mut HashSet<usize>, depth: usize) -> String {
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
    pub(super) fn eq(&self, b: &V) -> bool {
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
pub(super) fn value_matches(expected: Type, value: &V) -> bool {
    match expected {
        Type::Any => true,
        Type::Number => matches!(value, V::Num(_)),
        Type::String => matches!(value, V::Str(_)),
        Type::Bool => matches!(value, V::Bool(_)),
        Type::Null => matches!(value, V::Null),
        Type::Array => matches!(value, V::Arr(_)),
    }
}

pub(super) fn ensure_contract(expected: Type, value: &V, boundary: &str) -> std::result::Result<(), String> {
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

pub(super) fn contains_array_reference(value: &V, target: usize, visited: &mut HashSet<usize>) -> bool {
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
