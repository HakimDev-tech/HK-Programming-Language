//! Shared lexical environment and binding/contract semantics.
use super::types::Type;
use super::value::{ensure_contract, V};
use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    rc::{Rc, Weak},
};

#[derive(Debug)]
pub(super) enum EnvError {
    Duplicate(String),
    Constant(String),
    ImportedAlias(String),
    Undefined(String),
    Contract(String),
}

#[derive(Default)]
pub(super) struct Env {
    values: HashMap<String, V>,
    constants: HashSet<String>,
    contracts: HashMap<String, Type>,
    function_names: HashSet<String>,
    aliases: HashMap<String, (Weak<RefCell<Env>>, String)>,
    parent: Option<Rc<RefCell<Env>>>,
}
impl Env {
    pub(super) fn ensure_print(environment: &Rc<RefCell<Self>>) {
        environment
            .borrow_mut()
            .values
            .entry("print".to_owned())
            .or_insert(V::Print);
    }

    pub(super) fn parent(e: &Rc<RefCell<Self>>) -> Option<Rc<RefCell<Self>>> {
        e.borrow().parent.clone()
    }

    pub(super) fn child(parent: Rc<RefCell<Env>>) -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(Self {
            parent: Some(parent),
            ..Self::default()
        }))
    }
    pub(super) fn define(
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
    pub(super) fn define_alias(
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
    pub(super) fn define_function(&mut self, name: String, value: V) -> std::result::Result<(), EnvError> {
        self.define(name.clone(), value, false, None, "function declaration")?;
        self.function_names.insert(name);
        Ok(())
    }
    pub(super) fn is_function_binding(e: &Rc<RefCell<Self>>, n: &str) -> bool {
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
    pub(super) fn has_contract(e: &Rc<RefCell<Self>>, n: &str) -> bool {
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
    pub(super) fn get(e: &Rc<RefCell<Self>>, n: &str) -> Option<V> {
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
    pub(super) fn set(e: &Rc<RefCell<Self>>, n: &str, v: V) -> std::result::Result<(), EnvError> {
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
                return Err(EnvError::ImportedAlias(format!(
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
