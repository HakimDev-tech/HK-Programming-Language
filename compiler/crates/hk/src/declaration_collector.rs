//! Deterministic module-level declaration collection.
//!
//! This pass builds the module namespace before linking or executing initializers.
//! It intentionally collects only declaration forms supported by the current
//! executable frontend; v0.5 nominal declarations will extend this model.
use crate::ast::S;
use crate::{HkError, Result};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SymbolKind {
    Import,
    Variable,
    Function,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Symbol {
    pub(crate) kind: SymbolKind,
    pub(crate) exported: bool,
    pub(crate) line: usize,
    pub(crate) column: usize,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct ModuleSymbols {
    /// Sorted by name so downstream passes never depend on hash iteration.
    pub(crate) symbols: BTreeMap<String, Symbol>,
}

impl ModuleSymbols {
    pub(crate) fn exported_names(&self) -> impl Iterator<Item = &str> {
        self.symbols
            .iter()
            .filter_map(|(name, symbol)| symbol.exported.then_some(name.as_str()))
    }
}

/// Collect module-level bindings and reject namespace collisions before any
/// module initializer is run. Imports, declarations and the built-in print
/// binding share one namespace.
pub(crate) fn collect(
    statements: &[S],
    source_name: &str,
    source_text: &str,
) -> Result<ModuleSymbols> {
    let mut symbols = ModuleSymbols::default();
    insert(
        &mut symbols,
        "print",
        Symbol {
            kind: SymbolKind::Import,
            exported: false,
            line: 1,
            column: 1,
        },
        source_name,
        source_text,
    )?;

    for statement in statements {
        match statement {
            S::Import(_, names, _, _) => {
                for name in names {
                    insert(
                        &mut symbols,
                        &name.local,
                        Symbol {
                            kind: SymbolKind::Import,
                            exported: false,
                            line: name.line,
                            column: name.column,
                        },
                        source_name,
                        source_text,
                    )?;
                }
            }
            S::Export(inner) => {
                if let Some((name, kind, (line, column))) = declaration(inner) {
                    insert(
                        &mut symbols,
                        name,
                        Symbol {
                            kind,
                            exported: true,
                            line,
                            column,
                        },
                        source_name,
                        source_text,
                    )?;
                }
            }
            other => {
                if let Some((name, kind, (line, column))) = declaration(other) {
                    insert(
                        &mut symbols,
                        name,
                        Symbol {
                            kind,
                            exported: false,
                            line,
                            column,
                        },
                        source_name,
                        source_text,
                    )?;
                }
            }
        }
    }
    Ok(symbols)
}

fn declaration(statement: &S) -> Option<(&str, SymbolKind, (usize, usize))> {
    match statement {
        S::Var(name, ..) | S::TypedVar(name, ..) => {
            Some((name, SymbolKind::Variable, statement.source_position()))
        }
        S::Fn(name, ..) | S::TypedFn(name, ..) => {
            Some((name, SymbolKind::Function, statement.source_position()))
        }
        _ => None,
    }
}

fn insert(
    symbols: &mut ModuleSymbols,
    name: &str,
    symbol: Symbol,
    source_name: &str,
    source_text: &str,
) -> Result<()> {
    if symbols.symbols.contains_key(name) {
        return Err(HkError::new(
            "E4001",
            format!("Duplicate module binding '{name}'."),
            source_name,
            symbol.line,
            symbol.column,
            source_text,
        ));
    }
    symbols.symbols.insert(name.to_owned(), symbol);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{lexer::Lexer, parser::Parser};

    fn parse(source: &str) -> Vec<S> {
        let tokens = Lexer::new("collect.hk", source).scan().unwrap();
        Parser::new(tokens, "collect.hk", source).parse().unwrap()
    }

    #[test]
    fn collects_imports_declarations_and_exports_deterministically() {
        let statements = parse(
            r#"import { value as alias } from "./dep.hk"; export fn main() { return alias; } let local = 1;"#,
        );
        let symbols = collect(&statements, "collect.hk", "").unwrap();
        assert_eq!(
            symbols.symbols.keys().map(String::as_str).collect::<Vec<_>>(),
            vec!["alias", "local", "main", "print"]
        );
        assert_eq!(symbols.exported_names().collect::<Vec<_>>(), vec!["main"]);
        assert_eq!(symbols.symbols["main"].kind, SymbolKind::Function);
    }

    #[test]
    fn rejects_collisions_between_imports_and_declarations() {
        let statements = parse(r#"import { value as item } from "./dep.hk"; let item = 1;"#);
        let error = collect(&statements, "collect.hk", "").unwrap_err();
        assert_eq!(error.code, "E4001");
        assert!(error.message.contains("item"));
    }

    #[test]
    fn rejects_reserved_print_binding() {
        let statements = parse("let print = 1;");
        assert_eq!(collect(&statements, "collect.hk", "").unwrap_err().code, "E4001");
    }
}
