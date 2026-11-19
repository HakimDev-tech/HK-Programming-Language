//! Experimental, source-spanned front-end model for the HK 0.5 grammar.
//!
//! This module is deliberately separate from the executable 0.4 parser until
//! declaration collection, type checking, bytecode lowering, and VM support
//! can be integrated atomically. It provides the lossless lexical contract and
//! recursive type-expression model used by the staged 0.5 parser.
use std::collections::HashSet;
use std::ops::Range;

const MAX_V05_SOURCE_BYTES: usize = 1_048_576;
const MAX_V05_TOKENS: usize = 100_000;
const MAX_V05_GENERIC_PARAMETERS: usize = 256;
const MAX_V05_TRAIT_BOUNDS_PER_PARAMETER: usize = 256;

/// Keywords introduced by HK 0.5.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum V05Keyword {
    Struct,
    Enum,
    Trait,
    Impl,
    Match,
}

/// Punctuation needed by nominal declarations and generic syntax.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum V05Punctuation {
    LeftBrace,
    RightBrace,
    LeftParen,
    RightParen,
    LeftBracket,
    RightBracket,
    Colon,
    Semicolon,
    Comma,
    Dot,
    DoubleColon,
    Less,
    Greater,
    Plus,
    Equal,
    Arrow,
    Underscore,
}

/// A token retains its exact UTF-8 byte range in the original source.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum V05TokenKind {
    Identifier(String),
    Keyword(V05Keyword),
    Punctuation(V05Punctuation),
    Number(String),
    StringLiteral(String),
    ExistingSyntax(String),
}

/// A generic-call inference diagnostic with the source span that introduced
/// the failed constraint.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct V05InferenceError {
    pub message: String,
    pub span: Range<usize>,
}

/// Infers a generic function's concrete type arguments from typed argument
/// expressions and an optional expected return type. Explicit type arguments
/// constrain the same solver. This API is staged and is not yet called by HK's
/// executable parser/compiler.
pub fn infer_v05_function_type_arguments(
    function: &V05FunctionDecl,
    argument_types: &[TypeExpr],
    expected_return: Option<&TypeExpr>,
    explicit_type_arguments: Option<&[TypeExpr]>,
    declarations: &[V05TopLevelDeclaration],
) -> Result<Vec<TypeExpr>, V05InferenceError> {
    let generic_names: HashSet<String> = function
        .generics
        .iter()
        .map(|bound| bound.parameter.clone())
        .collect();
    if argument_types.len() != function.parameters.len() {
        return Err(V05InferenceError {
            message: format!(
                "Function '{}' expects {} argument(s), found {}.",
                function.name,
                function.parameters.len(),
                argument_types.len()
            ),
            span: function.span.clone(),
        });
    }
    if let Some(explicit) = explicit_type_arguments {
        if explicit.len() != function.generics.len() {
            return Err(V05InferenceError {
                message: format!(
                    "Function '{}' expects {} explicit type argument(s), found {}.",
                    function.name,
                    function.generics.len(),
                    explicit.len()
                ),
                span: function.span.clone(),
            });
        }
    }

    let mut substitutions = std::collections::HashMap::<String, TypeExpr>::new();
    if let Some(explicit) = explicit_type_arguments {
        for (parameter, argument) in function.generics.iter().zip(explicit) {
            substitutions.insert(parameter.parameter.clone(), argument.clone());
        }
    }

    for (parameter, argument) in function.parameters.iter().zip(argument_types) {
        infer_v05_constraint(
            &parameter.ty,
            argument,
            &generic_names,
            &mut substitutions,
        )?;
    }
    if let (Some(return_pattern), Some(expected)) =
        (&function.return_type, expected_return)
    {
        infer_v05_constraint(
            return_pattern,
            expected,
            &generic_names,
            &mut substitutions,
        )?;
    }

    let mut inferred = Vec::with_capacity(function.generics.len());
    for generic in &function.generics {
        let Some(argument) = substitutions.get(&generic.parameter).cloned() else {
            return Err(V05InferenceError {
                message: format!(
                    "Cannot infer generic parameter '{}'; provide explicit type arguments or an expected return type.",
                    generic.parameter
                ),
                span: generic.span.clone(),
            });
        };
        for trait_name in &generic.traits {
            if !v05_type_satisfies_trait(&argument, trait_name, declarations) {
                return Err(V05InferenceError {
                    message: format!(
                        "Inferred type '{}' does not satisfy trait bound '{}'.",
                        canonical_type(&argument),
                        trait_name
                    ),
                    span: generic.span.clone(),
                });
            }
        }
        inferred.push(argument);
    }
    Ok(inferred)
}

fn infer_v05_constraint(
    pattern: &TypeExpr,
    actual: &TypeExpr,
    generic_parameters: &HashSet<String>,
    substitutions: &mut std::collections::HashMap<String, TypeExpr>,
) -> Result<(), V05InferenceError> {
    match pattern {
        TypeExpr::Any { .. } => Ok(()),
        TypeExpr::Named {
            name,
            arguments,
            span,
        } if generic_parameters.contains(name) && arguments.is_empty() => {
            if is_v05_any_type(actual) {
                return Ok(());
            }
            if let Some(existing) = substitutions.get(name) {
                if same_type_expr(existing, actual) {
                    Ok(())
                } else {
                    Err(V05InferenceError {
                        message: format!(
                            "Conflicting constraints for generic parameter '{name}': '{}' and '{}'.",
                            canonical_type(existing),
                            canonical_type(actual)
                        ),
                        span: span.clone(),
                    })
                }
            } else {
                substitutions.insert(name.clone(), actual.clone());
                Ok(())
            }
        }
        TypeExpr::Named {
            name: pattern_name,
            arguments: pattern_arguments,
            span,
        } => match actual {
            TypeExpr::Any { .. } => Ok(()),
            TypeExpr::Named {
                name: actual_name,
                arguments: actual_arguments,
                ..
            } if pattern_name == actual_name && pattern_arguments.len() == actual_arguments.len() => {
                for (pattern_argument, actual_argument) in
                    pattern_arguments.iter().zip(actual_arguments)
                {
                    infer_v05_constraint(
                        pattern_argument,
                        actual_argument,
                        generic_parameters,
                        substitutions,
                    )?;
                }
                Ok(())
            }
            _ => Err(V05InferenceError {
                message: format!(
                    "Type constraint mismatch: expected '{}', found '{}'.",
                    canonical_type(pattern),
                    canonical_type(actual)
                ),
                span: span.clone(),
            }),
        },
    }
}

fn is_v05_any_type(ty: &TypeExpr) -> bool {
    match ty {
        TypeExpr::Any { .. } => true,
        TypeExpr::Named { name, arguments, .. } => name == "any" && arguments.is_empty(),
    }
}

fn same_type_expr(left: &TypeExpr, right: &TypeExpr) -> bool {
    canonical_type(left) == canonical_type(right)
}

fn v05_type_satisfies_trait(
    argument: &TypeExpr,
    trait_name: &str,
    declarations: &[V05TopLevelDeclaration],
) -> bool {
    if matches!(argument, TypeExpr::Any { .. })
        || matches!(argument, TypeExpr::Named { name, arguments, .. } if name == "any" && arguments.is_empty())
    {
        return false;
    }
    declarations.iter().any(|declaration| {
        let V05TopLevelDeclaration::Impl(implementation) = declaration else {
            return false;
        };
        if implementation.trait_name.as_deref() != Some(trait_name) {
            return false;
        }
        let generic_names: HashSet<String> = implementation
            .generics
            .iter()
            .map(|bound| bound.parameter.clone())
            .collect();
        type_pattern_matches(
            &implementation.target,
            &generic_names,
            argument,
        )
    })
}

fn type_pattern_matches(
    pattern: &TypeExpr,
    generic_parameters: &HashSet<String>,
    actual: &TypeExpr,
) -> bool {
    match pattern {
        TypeExpr::Any { .. } => false,
        TypeExpr::Named {
            name,
            arguments,
            ..
        } if generic_parameters.contains(name) && arguments.is_empty() => true,
        TypeExpr::Named {
            name: pattern_name,
            arguments: pattern_arguments,
            ..
        } => match actual {
            TypeExpr::Named {
                name: actual_name,
                arguments: actual_arguments,
                ..
            } if pattern_name == actual_name && pattern_arguments.len() == actual_arguments.len() => {
                pattern_arguments
                    .iter()
                    .zip(actual_arguments)
                    .all(|(pattern, actual)| {
                        type_pattern_matches(pattern, generic_parameters, actual)
                    })
            }
            _ => false,
        },
    }
}

/// Module declaration symbol category.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum V05SymbolKind {
    Struct,
    Enum,
    Trait,
    Function,
}

/// A declaration's name and source span in the staged module index.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct V05Symbol {
    pub kind: V05SymbolKind,
    pub span: Range<usize>,
}

/// Result of the staged declaration-front-end pipeline. This deliberately does
/// not represent executable AST or claim that expressions/method bodies have
/// been parsed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct V05ModuleIndex {
    pub declarations: Vec<V05TopLevelDeclaration>,
    pub symbols: std::collections::BTreeMap<String, V05Symbol>,
}

/// Error propagated by the declaration analysis entry point.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum V05ModuleError {
    Lex(V05LexError),
    Parse(V05ParseError),
}

impl std::fmt::Display for V05ModuleError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Lex(error) => write!(formatter, "{}", error.message),
            Self::Parse(error) => write!(formatter, "{}", error.message),
        }
    }
}

impl std::error::Error for V05ModuleError {}

/// Runs the staged v0.5 declaration frontend as a single ordered pipeline:
/// lexical analysis, mixed declaration parsing, type-name/generic-arity
/// validation, trait coherence, and deterministic symbol indexing.
pub fn analyze_v05_declaration_module(source: &str) -> Result<V05ModuleIndex, V05ModuleError> {
    let tokens = lex_v05(source).map_err(V05ModuleError::Lex)?;
    let declarations = parse_v05_top_level_declarations(&tokens).map_err(V05ModuleError::Parse)?;
    validate_v05_type_names(&declarations, &tokens).map_err(V05ModuleError::Parse)?;
    validate_v05_coherence(&declarations, &tokens).map_err(V05ModuleError::Parse)?;

    let mut symbols = std::collections::BTreeMap::new();
    for declaration in &declarations {
        let Some(name) = declaration.name() else {
            continue;
        };
        let kind = match declaration {
            V05TopLevelDeclaration::Struct(_) => V05SymbolKind::Struct,
            V05TopLevelDeclaration::Enum(_) => V05SymbolKind::Enum,
            V05TopLevelDeclaration::Trait(_) => V05SymbolKind::Trait,
            V05TopLevelDeclaration::Function(_) => V05SymbolKind::Function,
            V05TopLevelDeclaration::Impl(_) => unreachable!("impl blocks are not named symbols"),
        };
        symbols.insert(
            name.to_owned(),
            V05Symbol {
                kind,
                span: declaration.span(),
            },
        );
    }
    Ok(V05ModuleIndex {
        declarations,
        symbols,
    })
}

/// Unified top-level declaration model for mixed HK 0.5 source files.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum V05TopLevelDeclaration {
    Struct(V05StructDecl),
    Enum(V05EnumDecl),
    Trait(V05TraitDecl),
    Impl(V05ImplDecl),
    Function(V05FunctionDecl),
}

impl V05TopLevelDeclaration {
    pub fn name(&self) -> Option<&str> {
        match self {
            Self::Struct(declaration) => Some(&declaration.name),
            Self::Enum(declaration) => Some(&declaration.name),
            Self::Trait(declaration) => Some(&declaration.name),
            Self::Function(declaration) => Some(&declaration.name),
            Self::Impl(_) => None,
        }
    }

    pub fn span(&self) -> Range<usize> {
        match self {
            Self::Struct(declaration) => declaration.span.clone(),
            Self::Enum(declaration) => declaration.span.clone(),
            Self::Trait(declaration) => declaration.span.clone(),
            Self::Impl(declaration) => declaration.span.clone(),
            Self::Function(declaration) => declaration.span.clone(),
        }
    }
}

/// Performs declaration-level type-name and generic-arity validation over the
/// staged v0.5 declaration AST. Resolution is module-local at this stage;
/// imports and executable type checking are handled by later integration work.
pub fn validate_v05_type_names(
    declarations: &[V05TopLevelDeclaration],
    tokens: &[V05Token],
) -> Result<(), V05ParseError> {
    use std::collections::{HashMap, HashSet};

    let nominal: HashMap<&str, usize> = declarations
        .iter()
        .filter_map(|declaration| match declaration {
            V05TopLevelDeclaration::Struct(value) => {
                Some((value.name.as_str(), value.generics.len()))
            }
            V05TopLevelDeclaration::Enum(value) => {
                Some((value.name.as_str(), value.generics.len()))
            }
            _ => None,
        })
        .collect();
    let traits: HashSet<&str> = declarations
        .iter()
        .filter_map(|declaration| match declaration {
            V05TopLevelDeclaration::Trait(value) => Some(value.name.as_str()),
            _ => None,
        })
        .collect();

    for declaration in declarations {
        match declaration {
            V05TopLevelDeclaration::Struct(value) => {
                let parameters = generic_parameters(&value.generics, tokens, &traits)?;
                for field in &value.fields {
                    validate_type_expr_names(&field.ty, &parameters, &nominal, tokens)?;
                }
            }
            V05TopLevelDeclaration::Enum(value) => {
                let parameters = generic_parameters(&value.generics, tokens, &traits)?;
                for variant in &value.variants {
                    for payload in &variant.payload {
                        validate_type_expr_names(payload, &parameters, &nominal, tokens)?;
                    }
                }
            }
            V05TopLevelDeclaration::Trait(value) => {
                for method in &value.methods {
                    validate_method_type_names(method, &HashSet::new(), &nominal, tokens)?;
                }
            }
            V05TopLevelDeclaration::Function(value) => {
                let parameters = generic_parameters(&value.generics, tokens, &traits)?;
                for parameter in &value.parameters {
                    validate_type_expr_names(&parameter.ty, &parameters, &nominal, tokens)?;
                }
                if let Some(return_type) = &value.return_type {
                    validate_type_expr_names(return_type, &parameters, &nominal, tokens)?;
                }
            }
            V05TopLevelDeclaration::Impl(value) => {
                let parameters = generic_parameters(&value.generics, tokens, &traits)?;
                validate_type_expr_names(&value.target, &parameters, &nominal, tokens)?;
                if let Some(trait_name) = &value.trait_name {
                    if !traits.contains(trait_name.as_str()) {
                        return Err(parse_error_at_span(
                            tokens,
                            value.span.clone(),
                            &format!("Unknown trait name '{trait_name}'."),
                        ));
                    }
                }
                for method in &value.methods {
                    validate_method_type_names(method, &parameters, &nominal, tokens)?;
                }
            }
        }
    }
    Ok(())
}

fn generic_parameters(
    bounds: &[GenericBound],
    tokens: &[V05Token],
    traits: &std::collections::HashSet<&str>,
) -> Result<std::collections::HashSet<String>, V05ParseError> {
    let mut names = std::collections::HashSet::new();
    for bound in bounds {
        if !names.insert(bound.parameter.clone()) {
            return Err(parse_error_at_span(
                tokens,
                bound.span.clone(),
                "Duplicate generic parameter.",
            ));
        }
        for trait_name in &bound.traits {
            if !traits.contains(trait_name.as_str()) {
                return Err(parse_error_at_span(
                    tokens,
                    bound.span.clone(),
                    &format!("Unknown trait bound '{trait_name}'."),
                ));
            }
        }
    }
    Ok(names)
}

fn validate_method_type_names(
    method: &V05Method,
    parameters: &std::collections::HashSet<String>,
    nominal: &std::collections::HashMap<&str, usize>,
    tokens: &[V05Token],
) -> Result<(), V05ParseError> {
    for parameter in &method.parameters {
        if let Some(ty) = &parameter.ty {
            validate_type_expr_names(ty, parameters, nominal, tokens)?;
        }
    }
    if let Some(return_type) = &method.return_type {
        validate_type_expr_names(return_type, parameters, nominal, tokens)?;
    }
    Ok(())
}

fn validate_type_expr_names(
    ty: &TypeExpr,
    parameters: &std::collections::HashSet<String>,
    nominal: &std::collections::HashMap<&str, usize>,
    tokens: &[V05Token],
) -> Result<(), V05ParseError> {
    match ty {
        TypeExpr::Any { .. } => Ok(()),
        TypeExpr::Named {
            name,
            arguments,
            span,
        } => {
            let primitive = matches!(
                name.as_str(),
                "number" | "string" | "bool" | "null" | "array" | "any"
            );
            if parameters.contains(name) {
                if !arguments.is_empty() {
                    return Err(parse_error_at_span(
                        tokens,
                        span.clone(),
                        "Generic parameter cannot be applied to type arguments.",
                    ));
                }
            } else if primitive {
                if !arguments.is_empty() {
                    return Err(parse_error_at_span(
                        tokens,
                        span.clone(),
                        "Primitive type cannot take generic arguments.",
                    ));
                }
            } else if let Some(expected) = nominal.get(name.as_str()) {
                if arguments.len() != *expected {
                    return Err(parse_error_at_span(
                        tokens,
                        span.clone(),
                        &format!(
                            "Type '{name}' expects {expected} generic argument(s), found {}.",
                            arguments.len()
                        ),
                    ));
                }
            } else {
                return Err(parse_error_at_span(
                    tokens,
                    span.clone(),
                    &format!("Unknown type name '{name}'."),
                ));
            }
            for argument in arguments {
                validate_type_expr_names(argument, parameters, nominal, tokens)?;
            }
            Ok(())
        }
    }
}

#[derive(Clone, Debug)]
struct V05ImplMethodSet {
    target: TypeExpr,
    generic_parameters: HashSet<String>,
    methods: HashSet<String>,
}

/// Validates module-local trait implementation coherence for a parsed declaration
/// stream. This is semantic groundwork; imported symbols must be supplied by the
/// module resolver before the production compiler can invoke this pass.
pub fn validate_v05_coherence(
    declarations: &[V05TopLevelDeclaration],
    tokens: &[V05Token],
) -> Result<(), V05ParseError> {
    use std::collections::{HashMap, HashSet};

    let local_types: HashSet<String> = declarations
        .iter()
        .filter_map(|declaration| match declaration {
            V05TopLevelDeclaration::Struct(value) => Some(value.name.clone()),
            V05TopLevelDeclaration::Enum(value) => Some(value.name.clone()),
            _ => None,
        })
        .collect();
    let traits: HashMap<String, &V05TraitDecl> = declarations
        .iter()
        .filter_map(|declaration| match declaration {
            V05TopLevelDeclaration::Trait(value) => Some((value.name.clone(), value)),
            _ => None,
        })
        .collect();

    let mut trait_impls: Vec<(String, TypeExpr, HashSet<String>, Range<usize>)> = Vec::new();
    let mut inherent_methods: Vec<V05ImplMethodSet> = Vec::new();
    let mut trait_methods_by_target: Vec<V05ImplMethodSet> = Vec::new();

    for declaration in declarations {
        let V05TopLevelDeclaration::Impl(implementation) = declaration else {
            continue;
        };
        let target_name = type_root_name(&implementation.target).ok_or_else(|| {
            parse_error_at_span(
                tokens,
                type_span(&implementation.target),
                "Invalid impl target type.",
            )
        })?;
        let generic_names: HashSet<String> = implementation
            .generics
            .iter()
            .map(|bound| bound.parameter.clone())
            .collect();

        if implementation.trait_name.is_none() && !local_types.contains(target_name) {
            return Err(parse_error_at_span(
                tokens,
                implementation.span.clone(),
                "Inherent impl target must be declared in this module.",
            ));
        }
        if let Some(trait_name) = &implementation.trait_name {
            if !traits.contains_key(trait_name) && !local_types.contains(target_name) {
                return Err(parse_error_at_span(
                    tokens,
                    implementation.span.clone(),
                    "Trait impl violates the orphan rule: neither trait nor target type is local.",
                ));
            }
            if trait_impls
                .iter()
                .any(|(existing_trait, existing_target, existing_generics, _)| {
                    existing_trait == trait_name
                        && type_patterns_overlap(
                            existing_target,
                            existing_generics,
                            &implementation.target,
                            &generic_names,
                        )
                })
            {
                return Err(parse_error_at_span(
                    tokens,
                    implementation.span.clone(),
                    "Overlapping trait implementation for the same trait and target type.",
                ));
            }
            trait_impls.push((
                trait_name.clone(),
                implementation.target.clone(),
                generic_names.clone(),
                implementation.span.clone(),
            ));
        }

        let mut current_names = HashSet::new();
        for method in &implementation.methods {
            if !current_names.insert(method.name.clone()) {
                return Err(parse_error_at_span(
                    tokens,
                    method.span.clone(),
                    "Duplicate method in impl block.",
                ));
            }
        }

        if implementation.trait_name.is_none() {
            for existing in &inherent_methods {
                if type_patterns_overlap(
                    &existing.target,
                    &existing.generic_parameters,
                    &implementation.target,
                    &generic_names,
                ) {
                    if let Some(method) = current_names.intersection(&existing.methods).next() {
                        return Err(parse_error_at_span(
                            tokens,
                            implementation.span.clone(),
                            &format!(
                                "Duplicate inherent method '{method}' for overlapping target types."
                            ),
                        ));
                    }
                }
            }
            for existing in &trait_methods_by_target {
                if type_patterns_overlap(
                    &existing.target,
                    &existing.generic_parameters,
                    &implementation.target,
                    &generic_names,
                ) {
                    if let Some(method) = current_names.intersection(&existing.methods).next() {
                        return Err(parse_error_at_span(
                            tokens,
                            implementation.span.clone(),
                            &format!("Inherent and trait method name '{method}' conflict."),
                        ));
                    }
                }
            }
            inherent_methods.push(V05ImplMethodSet {
                target: implementation.target.clone(),
                generic_parameters: generic_names,
                methods: current_names,
            });
        } else {
            for existing in &trait_methods_by_target {
                if type_patterns_overlap(
                    &existing.target,
                    &existing.generic_parameters,
                    &implementation.target,
                    &generic_names,
                ) {
                    if let Some(method) = current_names.intersection(&existing.methods).next() {
                        return Err(parse_error_at_span(
                            tokens,
                            implementation.span.clone(),
                            &format!(
                                "Duplicate trait method '{method}' for overlapping target types."
                            ),
                        ));
                    }
                }
            }
            for existing in &inherent_methods {
                if type_patterns_overlap(
                    &existing.target,
                    &existing.generic_parameters,
                    &implementation.target,
                    &generic_names,
                ) {
                    if let Some(method) = current_names.intersection(&existing.methods).next() {
                        return Err(parse_error_at_span(
                            tokens,
                            implementation.span.clone(),
                            &format!("Inherent and trait method name '{method}' conflict."),
                        ));
                    }
                }
            }
            trait_methods_by_target.push(V05ImplMethodSet {
                target: implementation.target.clone(),
                generic_parameters: generic_names,
                methods: current_names,
            });
        }

        if let Some(trait_name) = &implementation.trait_name {
            if let Some(trait_declaration) = traits.get(trait_name) {
                validate_trait_methods(trait_declaration, implementation, tokens)?;
            }
        }
    }

    Ok(())
}

fn validate_trait_methods(
    trait_declaration: &V05TraitDecl,
    implementation: &V05ImplDecl,
    tokens: &[V05Token],
) -> Result<(), V05ParseError> {
    use std::collections::HashMap;

    let declared: HashMap<&str, &V05Method> = trait_declaration
        .methods
        .iter()
        .map(|method| (method.name.as_str(), method))
        .collect();
    let provided: HashMap<&str, &V05Method> = implementation
        .methods
        .iter()
        .map(|method| (method.name.as_str(), method))
        .collect();

    for method in &trait_declaration.methods {
        let actual = provided.get(method.name.as_str()).ok_or_else(|| {
            parse_error_at_span(
                tokens,
                implementation.span.clone(),
                &format!("Missing trait method '{}'.", method.name),
            )
        })?;
        if !same_method_signature(method, actual) {
            return Err(parse_error_at_span(
                tokens,
                actual.span.clone(),
                &format!("Method '{}' does not match trait signature.", method.name),
            ));
        }
    }
    if let Some(extra) = provided.keys().find(|name| !declared.contains_key(**name)) {
        return Err(parse_error_at_span(
            tokens,
            provided[extra].span.clone(),
            &format!("Method '{}' is not declared by the trait.", extra),
        ));
    }
    Ok(())
}

fn same_method_signature(left: &V05Method, right: &V05Method) -> bool {
    left.parameters.len() == right.parameters.len()
        && left
            .parameters
            .iter()
            .zip(&right.parameters)
            .all(|(left, right)| {
                left.ty.as_ref().map(canonical_type) == right.ty.as_ref().map(canonical_type)
            })
        && left.return_type.as_ref().map(canonical_type)
            == right.return_type.as_ref().map(canonical_type)
}

fn canonical_type(ty: &TypeExpr) -> String {
    match ty {
        TypeExpr::Any { .. } => "any".to_owned(),
        TypeExpr::Named {
            name, arguments, ..
        } => {
            if arguments.is_empty() {
                name.clone()
            } else {
                format!(
                    "{}<{}>",
                    name,
                    arguments
                        .iter()
                        .map(canonical_type)
                        .collect::<Vec<_>>()
                        .join(",")
                )
            }
        }
    }
}

fn type_patterns_overlap(
    left: &TypeExpr,
    left_parameters: &std::collections::HashSet<String>,
    right: &TypeExpr,
    right_parameters: &std::collections::HashSet<String>,
) -> bool {
    match (left, right) {
        (TypeExpr::Any { .. }, _) | (_, TypeExpr::Any { .. }) => true,
        (
            TypeExpr::Named {
                name: left_name,
                arguments: left_arguments,
                ..
            },
            TypeExpr::Named {
                name: right_name,
                arguments: right_arguments,
                ..
            },
        ) => {
            let left_variable = left_parameters.contains(left_name) && left_arguments.is_empty();
            let right_variable =
                right_parameters.contains(right_name) && right_arguments.is_empty();
            if left_variable || right_variable {
                return true;
            }
            left_name == right_name
                && left_arguments.len() == right_arguments.len()
                && left_arguments
                    .iter()
                    .zip(right_arguments)
                    .all(|(left, right)| {
                        type_patterns_overlap(left, left_parameters, right, right_parameters)
                    })
        }
    }
}

fn type_span(ty: &TypeExpr) -> Range<usize> {
    match ty {
        TypeExpr::Named { span, .. } | TypeExpr::Any { span } => span.clone(),
    }
}

fn type_root_name(ty: &TypeExpr) -> Option<&str> {
    match ty {
        TypeExpr::Named { name, .. } => Some(name),
        TypeExpr::Any { .. } => None,
    }
}

fn parse_error_at_span(tokens: &[V05Token], span: Range<usize>, message: &str) -> V05ParseError {
    let token_index = tokens
        .iter()
        .position(|token| token.span.start >= span.start)
        .unwrap_or(tokens.len());
    parse_error(tokens, token_index, message)
}

/// Parses a mixed module declaration stream. Declaration bodies remain source
/// spans; this pass validates declaration grammar and module-level name
/// uniqueness without pretending to parse function expressions or type-check.
pub fn parse_v05_top_level_declarations(
    tokens: &[V05Token],
) -> Result<Vec<V05TopLevelDeclaration>, V05ParseError> {
    let mut declarations = Vec::new();
    let mut names = std::collections::HashSet::new();
    let mut cursor = 0usize;

    while cursor < tokens.len() {
        let end = declaration_end(tokens, cursor)?;
        let slice = &tokens[cursor..end];
        let declaration = match tokens.get(cursor).map(|token| &token.kind) {
            Some(V05TokenKind::Keyword(V05Keyword::Struct | V05Keyword::Enum)) => {
                match parse_nominal_declarations(slice)?.into_iter().next() {
                    Some(V05Declaration::Struct(value)) => V05TopLevelDeclaration::Struct(value),
                    Some(V05Declaration::Enum(value)) => V05TopLevelDeclaration::Enum(value),
                    None => return Err(parse_error(tokens, cursor, "Empty nominal declaration.")),
                }
            }
            Some(V05TokenKind::Keyword(V05Keyword::Trait)) => {
                match parse_trait_and_impl_declarations(slice)?.into_iter().next() {
                    Some(V05TraitOrImpl::Trait(value)) => V05TopLevelDeclaration::Trait(value),
                    _ => return Err(parse_error(tokens, cursor, "Expected trait declaration.")),
                }
            }
            Some(V05TokenKind::Keyword(V05Keyword::Impl)) => {
                match parse_trait_and_impl_declarations(slice)?.into_iter().next() {
                    Some(V05TraitOrImpl::Impl(value)) => V05TopLevelDeclaration::Impl(value),
                    _ => return Err(parse_error(tokens, cursor, "Expected impl declaration.")),
                }
            }
            Some(V05TokenKind::Identifier(name)) if name == "fn" => {
                match parse_function_declarations(slice)?.into_iter().next() {
                    Some(value) => V05TopLevelDeclaration::Function(value),
                    None => {
                        return Err(parse_error(
                            tokens,
                            cursor,
                            "Expected function declaration.",
                        ))
                    }
                }
            }
            _ => {
                return Err(parse_error(
                    tokens,
                    cursor,
                    "Expected a top-level HK 0.5 declaration.",
                ))
            }
        };

        if let Some(name) = declaration.name() {
            if !names.insert(name.to_owned()) {
                return Err(parse_error(
                    tokens,
                    cursor,
                    "Duplicate module-level declaration name.",
                ));
            }
        }
        declarations.push(declaration);
        cursor = end;
    }

    Ok(declarations)
}

fn declaration_end(tokens: &[V05Token], start: usize) -> Result<usize, V05ParseError> {
    let mut brace_depth = 0usize;
    let mut saw_body = false;
    for (index, token) in tokens.iter().enumerate().skip(start) {
        if is_punctuation(Some(token), V05Punctuation::LeftBrace) {
            saw_body = true;
            brace_depth += 1;
        } else if is_punctuation(Some(token), V05Punctuation::RightBrace) {
            if brace_depth == 0 {
                return Err(parse_error(tokens, index, "Unexpected closing brace."));
            }
            brace_depth -= 1;
            if saw_body && brace_depth == 0 {
                return Ok(index + 1);
            }
        }
    }
    Err(parse_error(
        tokens,
        tokens.len(),
        "Unterminated top-level declaration body.",
    ))
}

/// A source-spanned token. Ranges are half-open UTF-8 byte offsets.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct V05Token {
    pub kind: V05TokenKind,
    pub span: Range<usize>,
    pub line: usize,
    pub column: usize,
}

/// Lexes the 0.5 declaration/generic vocabulary while preserving existing
/// expression tokens for the established 0.4 lexer/parser to interpret later.
pub fn lex_v05(source: &str) -> Result<Vec<V05Token>, V05LexError> {
    let bytes = source.as_bytes();
    if bytes.len() > MAX_V05_SOURCE_BYTES {
        return Err(V05LexError {
            message: format!(
                "Maximum HK 0.5 source size ({} bytes) exceeded.",
                MAX_V05_SOURCE_BYTES
            ),
            span: 0..bytes.len(),
            line: 1,
            column: 1,
        });
    }
    let mut offset = 0;
    let mut line = 1;
    let mut column = 1;
    let mut tokens = Vec::new();

    while offset < bytes.len() {
        let start = offset;
        let start_line = line;
        let start_column = column;
        let ch = source[offset..]
            .chars()
            .next()
            .expect("valid UTF-8 boundary");

        if ch.is_whitespace() {
            advance_position(ch, &mut offset, &mut line, &mut column);
            continue;
        }
        if ch == '/' && bytes.get(offset + 1) == Some(&b'/') {
            while offset < bytes.len() {
                let current = source[offset..]
                    .chars()
                    .next()
                    .expect("valid UTF-8 boundary");
                advance_position(current, &mut offset, &mut line, &mut column);
                if current == '\n' {
                    break;
                }
            }
            continue;
        }

        let kind = if ch == '_' || ch.is_ascii_alphabetic() {
            advance_position(ch, &mut offset, &mut line, &mut column);
            while offset < bytes.len() {
                let next = source[offset..]
                    .chars()
                    .next()
                    .expect("valid UTF-8 boundary");
                if next == '_' || next.is_ascii_alphanumeric() {
                    advance_position(next, &mut offset, &mut line, &mut column);
                } else {
                    break;
                }
            }
            let text = &source[start..offset];
            match text {
                "struct" => V05TokenKind::Keyword(V05Keyword::Struct),
                "enum" => V05TokenKind::Keyword(V05Keyword::Enum),
                "trait" => V05TokenKind::Keyword(V05Keyword::Trait),
                "impl" => V05TokenKind::Keyword(V05Keyword::Impl),
                "match" => V05TokenKind::Keyword(V05Keyword::Match),
                "_" => V05TokenKind::Punctuation(V05Punctuation::Underscore),
                _ => V05TokenKind::Identifier(text.to_owned()),
            }
        } else if ch.is_ascii_digit() {
            advance_position(ch, &mut offset, &mut line, &mut column);
            while offset < bytes.len() && bytes[offset].is_ascii_digit() {
                advance_position(
                    source[offset..].chars().next().expect("ASCII digit"),
                    &mut offset,
                    &mut line,
                    &mut column,
                );
            }
            V05TokenKind::Number(source[start..offset].to_owned())
        } else if ch == '"' {
            advance_position(ch, &mut offset, &mut line, &mut column);
            let mut escaped = false;
            let mut closed = false;
            while offset < bytes.len() {
                let current = source[offset..]
                    .chars()
                    .next()
                    .expect("valid UTF-8 boundary");
                advance_position(current, &mut offset, &mut line, &mut column);
                if escaped {
                    escaped = false;
                } else if current == '\\' {
                    escaped = true;
                } else if current == '"' {
                    closed = true;
                    break;
                }
            }
            if !closed {
                return Err(V05LexError {
                    message: "Unterminated string literal.".into(),
                    span: start..offset,
                    line: start_line,
                    column: start_column,
                });
            }
            V05TokenKind::StringLiteral(source[start..offset].to_owned())
        } else {
            let (punctuation, width) =
                match source[start..].chars().take(2).collect::<String>().as_str() {
                    "::" => (Some(V05Punctuation::DoubleColon), 2),
                    "->" => (Some(V05Punctuation::Arrow), 2),
                    _ => (None, 0),
                };
            if let Some(punctuation) = punctuation {
                for _ in 0..width {
                    let current = source[offset..]
                        .chars()
                        .next()
                        .expect("valid UTF-8 boundary");
                    advance_position(current, &mut offset, &mut line, &mut column);
                }
                V05TokenKind::Punctuation(punctuation)
            } else if let Some(punctuation) = single_punctuation(ch) {
                advance_position(ch, &mut offset, &mut line, &mut column);
                V05TokenKind::Punctuation(punctuation)
            } else if "{}()[]:;,.<>+=-*/%!&|?".contains(ch) {
                advance_position(ch, &mut offset, &mut line, &mut column);
                V05TokenKind::ExistingSyntax(ch.to_string())
            } else {
                return Err(V05LexError {
                    message: format!("Unexpected character '{ch}'."),
                    span: start..start + ch.len_utf8(),
                    line: start_line,
                    column: start_column,
                });
            }
        };

        if tokens.len() >= MAX_V05_TOKENS {
            return Err(V05LexError {
                message: format!("Maximum HK 0.5 token count ({MAX_V05_TOKENS}) exceeded."),
                span: start..offset,
                line: start_line,
                column: start_column,
            });
        }
        tokens.push(V05Token {
            kind,
            span: start..offset,
            line: start_line,
            column: start_column,
        });
    }

    Ok(tokens)
}

fn single_punctuation(ch: char) -> Option<V05Punctuation> {
    Some(match ch {
        '{' => V05Punctuation::LeftBrace,
        '}' => V05Punctuation::RightBrace,
        '(' => V05Punctuation::LeftParen,
        ')' => V05Punctuation::RightParen,
        '[' => V05Punctuation::LeftBracket,
        ']' => V05Punctuation::RightBracket,
        ':' => V05Punctuation::Colon,
        ';' => V05Punctuation::Semicolon,
        ',' => V05Punctuation::Comma,
        '.' => V05Punctuation::Dot,
        '<' => V05Punctuation::Less,
        '>' => V05Punctuation::Greater,
        '+' => V05Punctuation::Plus,
        '=' => V05Punctuation::Equal,
        _ => return None,
    })
}

fn advance_position(ch: char, offset: &mut usize, line: &mut usize, column: &mut usize) {
    *offset += ch.len_utf8();
    if ch == '\n' {
        *line += 1;
        *column = 1;
    } else {
        *column += 1;
    }
}

/// A recursive type expression; semantic resolution is intentionally deferred.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TypeExpr {
    Named {
        name: String,
        arguments: Vec<TypeExpr>,
        span: Range<usize>,
    },
    Any {
        span: Range<usize>,
    },
}

/// A bounded trait constraint such as `T: Display + Debug`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GenericBound {
    pub parameter: String,
    pub traits: Vec<String>,
    pub span: Range<usize>,
}

/// A parser failure with a precise token location.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct V05ParseError {
    pub message: String,
    pub span: Range<usize>,
    pub line: usize,
    pub column: usize,
}

/// Parses one recursive type expression at `*cursor` and advances the cursor
/// only after successfully consuming the complete expression.
pub fn parse_type_expr(tokens: &[V05Token], cursor: &mut usize) -> Result<TypeExpr, V05ParseError> {
    let original = *cursor;
    match parse_type_expr_at(tokens, cursor, 0) {
        Ok(ty) => Ok(ty),
        Err(error) => {
            *cursor = original;
            Err(error)
        }
    }
}

fn parse_type_expr_at(
    tokens: &[V05Token],
    cursor: &mut usize,
    depth: usize,
) -> Result<TypeExpr, V05ParseError> {
    if depth >= 64 {
        return Err(parse_error(
            tokens,
            *cursor,
            "Maximum type-expression nesting depth (64) exceeded.",
        ));
    }

    let token = tokens
        .get(*cursor)
        .ok_or_else(|| parse_error(tokens, *cursor, "Expected a type name."))?;
    let name = match &token.kind {
        V05TokenKind::Identifier(name) => name.clone(),
        _ => return Err(parse_error(tokens, *cursor, "Expected a type name.")),
    };
    *cursor += 1;

    if name == "any" {
        return Ok(TypeExpr::Any {
            span: token.span.clone(),
        });
    }

    let mut arguments = Vec::new();
    if is_punctuation(tokens.get(*cursor), V05Punctuation::Less) {
        *cursor += 1;
        if is_punctuation(tokens.get(*cursor), V05Punctuation::Greater) {
            return Err(parse_error(
                tokens,
                *cursor,
                "Generic argument lists cannot be empty.",
            ));
        }
        loop {
            arguments.push(parse_type_expr_at(tokens, cursor, depth + 1)?);
            if is_punctuation(tokens.get(*cursor), V05Punctuation::Comma) {
                *cursor += 1;
                if is_punctuation(tokens.get(*cursor), V05Punctuation::Greater) {
                    return Err(parse_error(tokens, *cursor, "Expected a type after ','."));
                }
            } else {
                break;
            }
        }
        if !is_punctuation(tokens.get(*cursor), V05Punctuation::Greater) {
            return Err(parse_error(
                tokens,
                *cursor,
                "Expected '>' after generic arguments.",
            ));
        }
        *cursor += 1;
    }

    let end = tokens
        .get(cursor.saturating_sub(1))
        .map(|t| t.span.end)
        .unwrap_or(token.span.end);
    Ok(TypeExpr::Named {
        name,
        arguments,
        span: token.span.start..end,
    })
}

/// Parses generic parameters in `<T: Trait + Debug, U>` form.
pub fn parse_generic_parameters(
    tokens: &[V05Token],
    cursor: &mut usize,
) -> Result<Vec<GenericBound>, V05ParseError> {
    if !is_punctuation(tokens.get(*cursor), V05Punctuation::Less) {
        return Ok(Vec::new());
    }
    *cursor += 1;
    let mut parameters = Vec::new();
    let mut parameter_names = std::collections::HashSet::new();
    loop {
        let token = tokens
            .get(*cursor)
            .ok_or_else(|| parse_error(tokens, *cursor, "Expected a generic parameter."))?;
        let parameter = match &token.kind {
            V05TokenKind::Identifier(name) => name.clone(),
            _ => {
                return Err(parse_error(
                    tokens,
                    *cursor,
                    "Expected a generic parameter name.",
                ))
            }
        };
        if !parameter_names.insert(parameter.clone()) {
            return Err(parse_error(tokens, *cursor, "Duplicate generic parameter."));
        }
        *cursor += 1;
        let mut traits = Vec::new();
        let mut trait_names = std::collections::HashSet::new();
        if is_punctuation(tokens.get(*cursor), V05Punctuation::Colon) {
            *cursor += 1;
            loop {
                let bound = tokens
                    .get(*cursor)
                    .ok_or_else(|| parse_error(tokens, *cursor, "Expected a trait bound."))?;
                match &bound.kind {
                    V05TokenKind::Identifier(name) => {
                        if !trait_names.insert(name.clone()) {
                            return Err(parse_error(tokens, *cursor, "Duplicate trait bound."));
                        }
                        traits.push(name.clone());
                    }
                    _ => {
                        return Err(parse_error(
                            tokens,
                            *cursor,
                            "Expected a trait name in generic bound.",
                        ))
                    }
                }
                *cursor += 1;
                if traits.len() > MAX_V05_TRAIT_BOUNDS_PER_PARAMETER {
                    return Err(parse_error(
                        tokens,
                        *cursor,
                        "Trait-bound list exceeds the supported limit (256).",
                    ));
                }
                if is_punctuation(tokens.get(*cursor), V05Punctuation::Plus) {
                    *cursor += 1;
                } else {
                    break;
                }
            }
        }
        let end = tokens
            .get(cursor.saturating_sub(1))
            .map(|t| t.span.end)
            .unwrap_or(token.span.end);
        parameters.push(GenericBound {
            parameter,
            traits,
            span: token.span.start..end,
        });
        if parameters.len() > MAX_V05_GENERIC_PARAMETERS {
            return Err(parse_error(
                tokens,
                *cursor,
                "Generic parameter list exceeds the supported limit (256).",
            ));
        }
        if is_punctuation(tokens.get(*cursor), V05Punctuation::Comma) {
            *cursor += 1;
            if is_punctuation(tokens.get(*cursor), V05Punctuation::Greater) {
                return Err(parse_error(
                    tokens,
                    *cursor,
                    "Expected a generic parameter after ','.",
                ));
            }
        } else {
            break;
        }
    }
    if !is_punctuation(tokens.get(*cursor), V05Punctuation::Greater) {
        return Err(parse_error(
            tokens,
            *cursor,
            "Expected '>' after generic parameters.",
        ));
    }
    *cursor += 1;
    Ok(parameters)
}

fn is_punctuation(token: Option<&V05Token>, expected: V05Punctuation) -> bool {
    matches!(token.map(|t| &t.kind), Some(V05TokenKind::Punctuation(actual)) if *actual == expected)
}

fn parse_error(tokens: &[V05Token], cursor: usize, message: &str) -> V05ParseError {
    let token = tokens.get(cursor).or_else(|| tokens.last());
    let (span, line, column) = token
        .map(|t| (t.span.clone(), t.line, t.column))
        .unwrap_or((0..0, 1, 1));
    V05ParseError {
        message: message.into(),
        span,
        line,
        column,
    }
}

/// A lexical failure with stable source coordinates.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct V05LexError {
    pub message: String,
    pub span: Range<usize>,
    pub line: usize,
    pub column: usize,
}

/// Parsed field of a nominal struct declaration.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct V05Field {
    pub name: String,
    pub ty: TypeExpr,
    pub span: Range<usize>,
}

/// Struct declaration AST. Semantic resolution (identity, duplicate names across
/// modules, bounds and field type validity) is deliberately a later compiler phase.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct V05StructDecl {
    pub name: String,
    pub generics: Vec<GenericBound>,
    pub fields: Vec<V05Field>,
    pub span: Range<usize>,
}

/// Enum variant with unit or positional payload.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct V05EnumVariant {
    pub name: String,
    pub payload: Vec<TypeExpr>,
    pub span: Range<usize>,
}

/// Enum declaration AST.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct V05EnumDecl {
    pub name: String,
    pub generics: Vec<GenericBound>,
    pub variants: Vec<V05EnumVariant>,
    pub span: Range<usize>,
}

/// The supported declaration subset parsed by this staged parser.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum V05Declaration {
    Struct(V05StructDecl),
    Enum(V05EnumDecl),
}

/// Parse consecutive struct and enum declarations. The parser rejects unsupported
/// top-level constructs instead of silently skipping tokens.
pub fn parse_nominal_declarations(
    tokens: &[V05Token],
) -> Result<Vec<V05Declaration>, V05ParseError> {
    let mut parser = V05NominalParser { tokens, cursor: 0 };
    let mut declarations = Vec::new();
    while parser.cursor < tokens.len() {
        let declaration = match parser.peek_keyword() {
            Some(V05Keyword::Struct) => parser.parse_struct().map(V05Declaration::Struct),
            Some(V05Keyword::Enum) => parser.parse_enum().map(V05Declaration::Enum),
            _ => Err(parser.error("Expected a struct or enum declaration.")),
        };
        declarations.push(declaration?);
    }
    Ok(declarations)
}

struct V05NominalParser<'a> {
    tokens: &'a [V05Token],
    cursor: usize,
}

impl V05NominalParser<'_> {
    fn peek_keyword(&self) -> Option<V05Keyword> {
        match self.tokens.get(self.cursor).map(|t| &t.kind) {
            Some(V05TokenKind::Keyword(keyword)) => Some(*keyword),
            _ => None,
        }
    }

    fn error(&self, message: &str) -> V05ParseError {
        parse_error(self.tokens, self.cursor, message)
    }

    fn expect_keyword(&mut self, keyword: V05Keyword, message: &str) -> Result<(), V05ParseError> {
        if self.peek_keyword() == Some(keyword) {
            self.cursor += 1;
            Ok(())
        } else {
            Err(self.error(message))
        }
    }

    fn expect_punctuation(
        &mut self,
        punctuation: V05Punctuation,
        message: &str,
    ) -> Result<(), V05ParseError> {
        if is_punctuation(self.tokens.get(self.cursor), punctuation) {
            self.cursor += 1;
            Ok(())
        } else {
            Err(self.error(message))
        }
    }

    fn identifier(&mut self, message: &str) -> Result<(String, Range<usize>), V05ParseError> {
        let token = self
            .tokens
            .get(self.cursor)
            .ok_or_else(|| self.error(message))?;
        if let V05TokenKind::Identifier(name) = &token.kind {
            let value = name.clone();
            let span = token.span.clone();
            self.cursor += 1;
            Ok((value, span))
        } else {
            Err(self.error(message))
        }
    }

    fn parse_struct(&mut self) -> Result<V05StructDecl, V05ParseError> {
        let start = self.tokens[self.cursor].span.start;
        self.expect_keyword(V05Keyword::Struct, "Expected 'struct'.")?;
        let (name, name_span) = self.identifier("Expected struct name.")?;
        let generics = parse_generic_parameters(self.tokens, &mut self.cursor)?;
        self.expect_punctuation(
            V05Punctuation::LeftBrace,
            "Expected '{' before struct fields.",
        )?;
        let mut fields = Vec::new();
        let mut names = std::collections::HashSet::new();
        while !is_punctuation(self.tokens.get(self.cursor), V05Punctuation::RightBrace) {
            if self.cursor >= self.tokens.len() {
                return Err(self.error("Unterminated struct declaration; expected '}'."));
            }
            let (field_name, field_span) = self.identifier("Expected field name.")?;
            if !names.insert(field_name.clone()) {
                return Err(V05ParseError {
                    message: format!("Duplicate struct field '{field_name}'."),
                    span: field_span.clone(),
                    line: self.tokens[self.cursor - 1].line,
                    column: self.tokens[self.cursor - 1].column,
                });
            }
            self.expect_punctuation(V05Punctuation::Colon, "Expected ':' after field name.")?;
            let ty = parse_type_expr(self.tokens, &mut self.cursor)?;
            let end = ty_span_end(&ty);
            fields.push(V05Field {
                name: field_name,
                ty,
                span: field_span.start..end,
            });
            if is_punctuation(self.tokens.get(self.cursor), V05Punctuation::Comma) {
                self.cursor += 1;
            } else if !is_punctuation(self.tokens.get(self.cursor), V05Punctuation::RightBrace) {
                return Err(self.error("Expected ',' or '}' after field declaration."));
            }
        }
        self.expect_punctuation(
            V05Punctuation::RightBrace,
            "Expected '}' after struct fields.",
        )?;
        let end = self.tokens[self.cursor - 1].span.end;
        let _ = name_span;
        Ok(V05StructDecl {
            name,
            generics,
            fields,
            span: start..end,
        })
    }

    fn parse_enum(&mut self) -> Result<V05EnumDecl, V05ParseError> {
        let start = self.tokens[self.cursor].span.start;
        self.expect_keyword(V05Keyword::Enum, "Expected 'enum'.")?;
        let (name, _) = self.identifier("Expected enum name.")?;
        let generics = parse_generic_parameters(self.tokens, &mut self.cursor)?;
        self.expect_punctuation(
            V05Punctuation::LeftBrace,
            "Expected '{' before enum variants.",
        )?;
        let mut variants = Vec::new();
        let mut names = std::collections::HashSet::new();
        while !is_punctuation(self.tokens.get(self.cursor), V05Punctuation::RightBrace) {
            if self.cursor >= self.tokens.len() {
                return Err(self.error("Unterminated enum declaration; expected '}'."));
            }
            let (variant_name, variant_span) = self.identifier("Expected enum variant name.")?;
            if !names.insert(variant_name.clone()) {
                return Err(V05ParseError {
                    message: format!("Duplicate enum variant '{variant_name}'."),
                    span: variant_span.clone(),
                    line: self.tokens[self.cursor - 1].line,
                    column: self.tokens[self.cursor - 1].column,
                });
            }
            let mut payload = Vec::new();
            if is_punctuation(self.tokens.get(self.cursor), V05Punctuation::LeftParen) {
                self.cursor += 1;
                if !is_punctuation(self.tokens.get(self.cursor), V05Punctuation::RightParen) {
                    loop {
                        payload.push(parse_type_expr(self.tokens, &mut self.cursor)?);
                        if is_punctuation(self.tokens.get(self.cursor), V05Punctuation::Comma) {
                            self.cursor += 1;
                            if is_punctuation(
                                self.tokens.get(self.cursor),
                                V05Punctuation::RightParen,
                            ) {
                                break;
                            }
                        } else {
                            break;
                        }
                    }
                }
                self.expect_punctuation(
                    V05Punctuation::RightParen,
                    "Expected ')' after enum payload.",
                )?;
            }
            let end = self.tokens[self.cursor.saturating_sub(1)].span.end;
            variants.push(V05EnumVariant {
                name: variant_name,
                payload,
                span: variant_span.start..end,
            });
            if is_punctuation(self.tokens.get(self.cursor), V05Punctuation::Comma) {
                self.cursor += 1;
            } else if !is_punctuation(self.tokens.get(self.cursor), V05Punctuation::RightBrace) {
                return Err(self.error("Expected ',' or '}' after enum variant."));
            }
        }
        self.expect_punctuation(
            V05Punctuation::RightBrace,
            "Expected '}' after enum variants.",
        )?;
        let end = self.tokens[self.cursor - 1].span.end;
        Ok(V05EnumDecl {
            name,
            generics,
            variants,
            span: start..end,
        })
    }
}

fn ty_span_end(ty: &TypeExpr) -> usize {
    match ty {
        TypeExpr::Named { span, .. } | TypeExpr::Any { span } => span.end,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn infers_generic_function_arguments_from_arguments_and_expected_return() {
        let source = "fn identity<T>(value: T) -> T { return value; } fn create<T>() -> T { return null; }";
        let tokens = lex_v05(source).unwrap();
        let declarations = parse_v05_top_level_declarations(&tokens).unwrap();
        let functions = declarations
            .iter()
            .filter_map(|declaration| match declaration {
                V05TopLevelDeclaration::Function(function) => Some(function.clone()),
                _ => None,
            })
            .collect::<Vec<_>>();
        let number = lex_v05("number").unwrap();
        let mut cursor = 0;
        let number_type = parse_type_expr(&number, &mut cursor).unwrap();
        let inferred = infer_v05_generic_function_type_arguments(
            &functions[0],
            &[number_type.clone()],
            None,
            None,
            &[],
        )
        .unwrap();
        assert_eq!(canonical_type(&inferred[0]), "number");

        let string = lex_v05("string").unwrap();
        cursor = 0;
        let string_type = parse_type_expr(&string, &mut cursor).unwrap();
        let inferred = infer_v05_generic_function_type_arguments(
            &functions[1],
            &[],
            Some(&string_type),
            None,
            &[],
        )
        .unwrap();
        assert_eq!(canonical_type(&inferred[0]), "string");
    }

    #[test]
    fn generic_inference_rejects_conflicts_ambiguity_and_unsatisfied_bounds() {
        let source = "fn pair<T>(left: T, right: T) -> T { return left; } fn create<T>() -> T { return null; } trait Display { fn display(self) -> string; } fn render<T: Display>(value: T) -> string { return value.display(); }";
        let tokens = lex_v05(source).unwrap();
        let functions = parse_function_declarations(&tokens).unwrap();
        let number = lex_v05("number").unwrap();
        let string = lex_v05("string").unwrap();
        let mut cursor = 0;
        let number_type = parse_type_expr(&number, &mut cursor).unwrap();
        cursor = 0;
        let string_type = parse_type_expr(&string, &mut cursor).unwrap();

        let conflict = infer_v05_generic_function_type_arguments(
            &functions[0],
            &[number_type.clone(), string_type.clone()],
            None,
            None,
            &[],
        )
        .unwrap_err();
        assert!(conflict.message.contains("Conflicting constraints"));

        let ambiguous = infer_v05_generic_function_type_arguments(
            &functions[1],
            &[],
            None,
            None,
            &[],
        )
        .unwrap_err();
        assert!(ambiguous.message.contains("Cannot infer generic parameter"));

        let bound = infer_v05_generic_function_type_arguments(
            &functions[2],
            &[number_type],
            None,
            None,
            &[],
        )
        .unwrap_err();
        assert!(bound.message.contains("does not satisfy trait bound"));
    }

    #[test]
    fn declaration_module_analysis_runs_all_frontend_stages_and_indexes_symbols() {
        let source = "struct Box<T> { value: T, } trait Display { fn display(self) -> string; } impl Display for Box<number> { fn display(self) -> string { return \"box\"; } } fn identity<T>(value: T) -> T { return value; }";
        let index = analyze_v05_declaration_module(source).unwrap();
        assert_eq!(
            index.symbols.keys().map(String::as_str).collect::<Vec<_>>(),
            vec!["Box", "Display", "identity"]
        );
        assert_eq!(index.symbols["Box"].kind, V05SymbolKind::Struct);
        assert_eq!(index.symbols["Display"].kind, V05SymbolKind::Trait);
        assert_eq!(index.symbols["identity"].kind, V05SymbolKind::Function);
        assert_eq!(index.declarations.len(), 4);

        let invalid = "struct Box<T> { value: T, } fn broken(value: Box) -> number { return 1; }";
        let error = analyze_v05_declaration_module(invalid).unwrap_err();
        assert!(error.to_string().contains("generic argument"));
    }

    #[test]
    fn validates_v05_type_names_and_generic_arity() {
        let valid = "struct Box<T> { value: T, } struct Wrapper { item: Box<number>, } enum Option<T> { Some(T), None, } trait Display { fn display(self) -> string; } fn identity<T>(value: T) -> T { return value; } fn render<T: Display>(value: T) -> string { return value.display(); } fn use_box(value: Box<number>) -> number { return 1; }";
        let tokens = lex_v05(valid).unwrap();
        let declarations = parse_v05_top_level_declarations(&tokens).unwrap();
        validate_v05_type_names(&declarations, &tokens).unwrap();

        let unknown = "struct Point { value: Missing, }";
        let tokens = lex_v05(unknown).unwrap();
        let declarations = parse_v05_top_level_declarations(&tokens).unwrap();
        assert!(validate_v05_type_names(&declarations, &tokens)
            .unwrap_err()
            .message
            .contains("Unknown type name"));

        let wrong_arity =
            "struct Box<T> { value: T, } fn use_box(value: Box) -> number { return 1; }";
        let tokens = lex_v05(wrong_arity).unwrap();
        let declarations = parse_v05_top_level_declarations(&tokens).unwrap();
        assert!(validate_v05_type_names(&declarations, &tokens)
            .unwrap_err()
            .message
            .contains("expects 1 generic argument"));

        let unknown_trait_impl = "struct Point { x: number, } impl MissingTrait for Point { fn display(self) -> string { return \"p\"; } }";
        let tokens = lex_v05(unknown_trait_impl).unwrap();
        let declarations = parse_v05_top_level_declarations(&tokens).unwrap();
        assert!(validate_v05_type_names(&declarations, &tokens)
            .unwrap_err()
            .message
            .contains("Unknown trait name"));

        let unknown_bound = "fn render<T: MissingTrait>(value: T) -> string { return \"\"; }";
        let tokens = lex_v05(unknown_bound).unwrap();
        let declarations = parse_v05_top_level_declarations(&tokens).unwrap();
        assert!(validate_v05_type_names(&declarations, &tokens)
            .unwrap_err()
            .message
            .contains("Unknown trait bound"));
    }

    #[test]
    fn validates_trait_impl_conformance_and_orphan_rule() {
        let source = "struct Point { x: number, } trait Display { fn display(self) -> string; } impl Display for Point { fn display(self) -> string { return \"point\"; } }";
        let tokens = lex_v05(source).unwrap();
        let declarations = parse_v05_top_level_declarations(&tokens).unwrap();
        validate_v05_coherence(&declarations, &tokens).unwrap();

        let missing = "struct Point { x: number, } trait Display { fn display(self) -> string; fn size(self) -> number; } impl Display for Point { fn display(self) -> string { return \"point\"; } }";
        let tokens = lex_v05(missing).unwrap();
        let declarations = parse_v05_top_level_declarations(&tokens).unwrap();
        let error = validate_v05_coherence(&declarations, &tokens).unwrap_err();
        assert!(error.message.contains("Missing trait method"));

        let orphan = "impl Display for Foreign { fn display(self) -> string { return \"x\"; } }";
        let tokens = lex_v05(orphan).unwrap();
        let declarations = parse_v05_top_level_declarations(&tokens).unwrap();
        let error = validate_v05_coherence(&declarations, &tokens).unwrap_err();
        assert!(error.message.contains("orphan rule"));
    }

    #[test]
    fn rejects_extra_trait_methods_and_inherent_trait_name_conflicts() {
        let extra = "struct Point { x: number, } trait Display { fn display(self) -> string; } impl Display for Point { fn display(self) -> string { return \"p\"; } fn extra(self) { } }";
        let tokens = lex_v05(extra).unwrap();
        let declarations = parse_v05_top_level_declarations(&tokens).unwrap();
        let error = validate_v05_coherence(&declarations, &tokens).unwrap_err();
        assert!(error.message.contains("not declared by the trait"));

        let conflict = "struct Point { x: number, } trait Display { fn display(self) -> string; } impl Point { fn display(self) -> string { return \"p\"; } } impl Display for Point { fn display(self) -> string { return \"p\"; } }";
        let tokens = lex_v05(conflict).unwrap();
        let declarations = parse_v05_top_level_declarations(&tokens).unwrap();
        let error = validate_v05_coherence(&declarations, &tokens).unwrap_err();
        assert!(error.message.contains("conflict"));
    }

    #[test]
    fn coherence_detects_overlapping_inherent_impl_method_sets() {
        let source = "struct Box<T> { value: T, } impl<T> Box<T> { fn get(self) -> T { return self.value; } } impl Box<number> { fn get(self) -> number { return self.value; } }";
        let tokens = lex_v05(source).unwrap();
        let declarations = parse_v05_top_level_declarations(&tokens).unwrap();
        let error = validate_v05_coherence(&declarations, &tokens).unwrap_err();
        assert!(error.message.contains("Duplicate inherent method"));
    }

    #[test]
    fn coherence_detects_overlapping_generic_trait_impls() {
        let overlap = "struct Box<T> { value: T, } trait Display { fn display(self) -> string; } impl<T> Display for Box<T> { fn display(self) -> string { return \"generic\"; } } impl Display for Box<number> { fn display(self) -> string { return \"number\"; } }";
        let tokens = lex_v05(overlap).unwrap();
        let declarations = parse_v05_top_level_declarations(&tokens).unwrap();
        let error = validate_v05_coherence(&declarations, &tokens).unwrap_err();
        assert!(error.message.contains("Overlapping trait implementation"));

        let disjoint = "struct Box<T> { value: T, } trait Display { fn display(self) -> string; } impl Display for Box<number> { fn display(self) -> string { return \"number\"; } } impl Display for Box<string> { fn display(self) -> string { return \"string\"; } }";
        let tokens = lex_v05(disjoint).unwrap();
        let declarations = parse_v05_top_level_declarations(&tokens).unwrap();
        validate_v05_coherence(&declarations, &tokens).unwrap();
    }

    #[test]
    fn rejects_duplicate_trait_impl_pairs_and_inherent_methods() {
        let duplicate_impl = "struct Point { x: number, } trait Display { fn display(self) -> string; } impl Display for Point { fn display(self) -> string { return \"a\"; } } impl Display for Point { fn display(self) -> string { return \"b\"; } }";
        let tokens = lex_v05(duplicate_impl).unwrap();
        let declarations = parse_v05_top_level_declarations(&tokens).unwrap();
        let error = validate_v05_coherence(&declarations, &tokens).unwrap_err();
        assert!(error.message.contains("Overlapping trait implementation"));

        let duplicate_inherent = "struct Point { x: number, } impl Point { fn move(self) { } } impl Point { fn move(self) { } }";
        let tokens = lex_v05(duplicate_inherent).unwrap();
        let declarations = parse_v05_top_level_declarations(&tokens).unwrap();
        let error = validate_v05_coherence(&declarations, &tokens).unwrap_err();
        assert!(error.message.contains("Duplicate inherent method"));
    }

    #[test]
    fn rejects_trait_impl_signature_mismatch() {
        let source = "struct Point { x: number, } trait Display { fn display(self) -> string; } impl Display for Point { fn display(self) -> number { return 1; } }";
        let tokens = lex_v05(source).unwrap();
        let declarations = parse_v05_top_level_declarations(&tokens).unwrap();
        let error = validate_v05_coherence(&declarations, &tokens).unwrap_err();
        assert!(error.message.contains("does not match trait signature"));
    }

    #[test]
    fn v05_lexer_enforces_source_and_token_limits() {
        let oversized_source = "x".repeat(MAX_V05_SOURCE_BYTES + 1);
        let error = lex_v05(&oversized_source).unwrap_err();
        assert!(error.message.contains("source size"));

        let many_tokens = std::iter::repeat_n("x", MAX_V05_TOKENS + 1)
            .collect::<Vec<_>>()
            .join(" ");
        let error = lex_v05(&many_tokens).unwrap_err();
        assert!(error.message.contains("token count"));
        assert_eq!(error.span.start, MAX_V05_TOKENS * 2);
    }

    #[test]
    fn parses_mixed_top_level_v05_declarations_and_tracks_spans() {
        let source = "struct Point { x: number, y: number, } enum Direction { North, South, } trait Display { fn display(self) -> string; } impl Display for Point { fn display(self) -> string { return \"point\"; } } fn identity<T>(value: T) -> T { return value; }";
        let tokens = lex_v05(source).unwrap();
        let declarations = parse_v05_top_level_declarations(&tokens).unwrap();
        assert_eq!(declarations.len(), 5);
        assert_eq!(
            declarations
                .iter()
                .filter_map(V05TopLevelDeclaration::name)
                .collect::<Vec<_>>(),
            vec!["Point", "Direction", "Display", "identity"]
        );
        assert!(declarations.iter().all(|declaration| {
            let span = declaration.span();
            span.start < span.end && source.get(span.clone()).is_some()
        }));
        assert!(matches!(declarations[3], V05TopLevelDeclaration::Impl(_)));
        assert!(matches!(
            declarations[4],
            V05TopLevelDeclaration::Function(_)
        ));
    }

    #[test]
    fn mixed_top_level_parser_rejects_duplicate_names_across_kinds() {
        let tokens = lex_v05("struct Value { field: number, } fn Value() { }").unwrap();
        let error = parse_v05_top_level_declarations(&tokens).unwrap_err();
        assert!(error.message.contains("Duplicate module-level"));
    }

    #[test]
    fn mixed_top_level_parser_rejects_unterminated_declarations() {
        let tokens = lex_v05("struct Broken { field: number,").unwrap();
        let error = parse_v05_top_level_declarations(&tokens).unwrap_err();
        assert!(error.message.contains("Unterminated"));
    }

    #[test]
    fn parses_structs_and_enum_unit_and_tuple_variants() {
        let tokens = lex_v05(
            "struct Box<T: Display> { value: T, } enum Result<T, E> { Ok(T), Err(E), Empty, }",
        )
        .unwrap();
        let declarations = parse_nominal_declarations(&tokens).unwrap();
        assert_eq!(declarations.len(), 2);
        match &declarations[0] {
            V05Declaration::Struct(decl) => {
                assert_eq!(decl.name, "Box");
                assert_eq!(decl.generics[0].traits, vec!["Display"]);
                assert_eq!(decl.fields[0].name, "value");
            }
            _ => panic!("expected struct AST"),
        }
        match &declarations[1] {
            V05Declaration::Enum(decl) => {
                assert_eq!(decl.name, "Result");
                assert_eq!(decl.variants.len(), 3);
                assert_eq!(decl.variants[0].payload.len(), 1);
                assert_eq!(decl.variants[2].payload.len(), 0);
            }
            _ => panic!("expected enum AST"),
        }
    }

    #[test]
    fn rejects_duplicate_fields_and_variants() {
        let duplicate_field = lex_v05("struct S { a: number, a: string }").unwrap();
        assert!(parse_nominal_declarations(&duplicate_field)
            .unwrap_err()
            .message
            .contains("Duplicate"));
        let duplicate_variant = lex_v05("enum E { A, A }").unwrap();
        assert!(parse_nominal_declarations(&duplicate_variant)
            .unwrap_err()
            .message
            .contains("Duplicate"));
    }

    #[test]
    fn nominal_parser_rejects_trailing_unknown_syntax_and_unclosed_blocks() {
        assert!(parse_nominal_declarations(&lex_v05("struct S {} fn f() {}").unwrap()).is_err());
        assert!(parse_nominal_declarations(&lex_v05("enum E { A").unwrap()).is_err());
    }

    #[test]
    fn generic_parameter_parser_rejects_duplicates_and_caps_lists() {
        let duplicate = lex_v05("<T, T>").unwrap();
        let mut cursor = 0;
        assert!(parse_generic_parameters(&duplicate, &mut cursor)
            .unwrap_err()
            .message
            .contains("Duplicate generic"));

        let repeated_bound = lex_v05("<T: Display + Display>").unwrap();
        let mut cursor = 0;
        assert!(parse_generic_parameters(&repeated_bound, &mut cursor)
            .unwrap_err()
            .message
            .contains("Duplicate trait"));

        let many = format!(
            "<{}>",
            (0..257)
                .map(|i| format!("T{i}"))
                .collect::<Vec<_>>()
                .join(",")
        );
        let tokens = lex_v05(&many).unwrap();
        let mut cursor = 0;
        assert!(parse_generic_parameters(&tokens, &mut cursor)
            .unwrap_err()
            .message
            .contains("supported limit"));
    }

    #[test]
    fn recognizes_v05_keywords_and_generic_punctuation() {
        let tokens = lex_v05("struct Box<T> { value: T } enum Result<T, E> { Ok(T) } impl<T> Box<T> {} trait Show {} match value {}").unwrap();
        assert!(tokens
            .iter()
            .any(|t| t.kind == V05TokenKind::Keyword(V05Keyword::Struct)));
        assert!(tokens
            .iter()
            .any(|t| t.kind == V05TokenKind::Keyword(V05Keyword::Enum)));
        assert!(tokens
            .iter()
            .any(|t| t.kind == V05TokenKind::Keyword(V05Keyword::Impl)));
        assert!(tokens
            .iter()
            .any(|t| t.kind == V05TokenKind::Keyword(V05Keyword::Trait)));
        assert!(tokens
            .iter()
            .any(|t| t.kind == V05TokenKind::Keyword(V05Keyword::Match)));
        assert!(tokens
            .iter()
            .any(|t| t.kind == V05TokenKind::Punctuation(V05Punctuation::Less)));
        assert!(tokens
            .iter()
            .any(|t| t.kind == V05TokenKind::Punctuation(V05Punctuation::Greater)));
    }

    #[test]
    fn preserves_utf8_byte_spans_and_line_columns() {
        let source = "// café\nstruct Cafe {}";
        let tokens = lex_v05(source).unwrap();
        let cafe = tokens
            .iter()
            .find(|t| matches!(&t.kind, V05TokenKind::Identifier(name) if name == "Cafe"))
            .unwrap();
        assert_eq!(cafe.line, 2);
        assert_eq!(cafe.column, 8);
        assert_eq!(&source[cafe.span.clone()], "Cafe");
    }

    #[test]
    fn reports_unterminated_strings_without_panicking() {
        let error = lex_v05("struct A { name: \"unfinished").unwrap_err();
        assert_eq!(error.line, 1);
        assert_eq!(error.column, 18);
        assert!(error.message.contains("Unterminated"));
    }

    #[test]
    fn token_spans_are_valid_utf8_boundaries() {
        let source = "struct Point { x: number }";
        for token in lex_v05(source).unwrap() {
            assert_eq!(
                &source[token.span.clone()].len(),
                &(token.span.end - token.span.start)
            );
        }
    }
}

// ---- Trait and impl declaration parsing (staged M1.1 slice) ----------------

/// A named, typed instance-method parameter. The receiver is represented as the
/// ordinary first parameter and validated by later semantic analysis.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct V05MethodParameter {
    pub name: String,
    pub ty: Option<TypeExpr>,
    pub span: Range<usize>,
}

/// Method signature/body shape. Trait methods have no body; impl methods retain
/// their body token range for a later expression-parser integration.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct V05Method {
    pub name: String,
    pub parameters: Vec<V05MethodParameter>,
    pub return_type: Option<TypeExpr>,
    pub body_span: Option<Range<usize>>,
    pub span: Range<usize>,
}

/// Trait declaration: method signatures only, per the normative v0.5 contract.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct V05TraitDecl {
    pub name: String,
    pub methods: Vec<V05Method>,
    pub span: Range<usize>,
}

/// An impl declaration. A missing trait name denotes an inherent impl.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct V05ImplDecl {
    pub generics: Vec<GenericBound>,
    pub target: TypeExpr,
    pub trait_name: Option<String>,
    pub methods: Vec<V05Method>,
    pub span: Range<usize>,
}

/// Parses trait signatures and impl method headers, preserving impl bodies as
/// token spans. This is syntax-only: bodies are not expression-parsed and no
/// declaration is integrated into the executable compiler by this API.
pub fn parse_trait_and_impl_declarations(
    tokens: &[V05Token],
) -> Result<Vec<V05TraitOrImpl>, V05ParseError> {
    let mut parser = V05TraitImplParser { tokens, cursor: 0 };
    let mut declarations = Vec::new();
    while parser.cursor < tokens.len() {
        let declaration = match parser.peek_keyword() {
            Some(V05Keyword::Trait) => parser.parse_trait().map(V05TraitOrImpl::Trait),
            Some(V05Keyword::Impl) => parser.parse_impl().map(V05TraitOrImpl::Impl),
            _ => Err(parser.error("Expected a trait or impl declaration.")),
        };
        declarations.push(declaration?);
    }
    Ok(declarations)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum V05TraitOrImpl {
    Trait(V05TraitDecl),
    Impl(V05ImplDecl),
}

struct V05TraitImplParser<'a> {
    tokens: &'a [V05Token],
    cursor: usize,
}

impl V05TraitImplParser<'_> {
    fn peek_keyword(&self) -> Option<V05Keyword> {
        match self.tokens.get(self.cursor)?.kind {
            V05TokenKind::Keyword(keyword) => Some(keyword),
            _ => None,
        }
    }

    fn error(&self, message: &str) -> V05ParseError {
        parse_error(self.tokens, self.cursor, message)
    }

    fn expect_identifier(&mut self, what: &str) -> Result<String, V05ParseError> {
        let token = self
            .tokens
            .get(self.cursor)
            .ok_or_else(|| self.error(what))?;
        if let V05TokenKind::Identifier(name) = &token.kind {
            self.cursor += 1;
            Ok(name.clone())
        } else {
            Err(self.error(what))
        }
    }

    fn expect_punctuation(
        &mut self,
        punctuation: V05Punctuation,
        message: &str,
    ) -> Result<(), V05ParseError> {
        if is_punctuation(self.tokens.get(self.cursor), punctuation) {
            self.cursor += 1;
            Ok(())
        } else {
            Err(self.error(message))
        }
    }

    fn parse_trait(&mut self) -> Result<V05TraitDecl, V05ParseError> {
        let start = self.tokens[self.cursor].span.start;
        self.cursor += 1; // trait
        let name = self.expect_identifier("Expected trait name.")?;
        self.expect_punctuation(V05Punctuation::LeftBrace, "Expected '{' after trait name.")?;
        let mut methods = Vec::new();
        let mut names = std::collections::HashSet::new();
        while !is_punctuation(self.tokens.get(self.cursor), V05Punctuation::RightBrace) {
            if self.cursor >= self.tokens.len() {
                return Err(self.error("Unterminated trait declaration."));
            }
            let method = self.parse_method(false)?;
            self.validate_receiver(&method)?;
            if !names.insert(method.name.clone()) {
                return Err(self.error("Duplicate trait method."));
            }
            methods.push(method);
        }
        self.cursor += 1;
        let end = self.tokens[self.cursor - 1].span.end;
        Ok(V05TraitDecl {
            name,
            methods,
            span: start..end,
        })
    }

    fn parse_impl(&mut self) -> Result<V05ImplDecl, V05ParseError> {
        let start = self.tokens[self.cursor].span.start;
        self.cursor += 1; // impl
        let generics = parse_generic_parameters(self.tokens, &mut self.cursor)?;
        let first = parse_type_expr(self.tokens, &mut self.cursor)?;
        let (trait_name, target) = if self
            .tokens
            .get(self.cursor)
            .is_some_and(|t| matches!(&t.kind, V05TokenKind::Identifier(s) if s == "for"))
        {
            self.cursor += 1;
            let target = parse_type_expr(self.tokens, &mut self.cursor)?;
            let trait_name = match first {
                TypeExpr::Named {
                    name, arguments, ..
                } if arguments.is_empty() => name,
                _ => {
                    return Err(self.error("Trait name must be a plain identifier."));
                }
            };
            (Some(trait_name), target)
        } else {
            (None, first)
        };
        self.expect_punctuation(
            V05Punctuation::LeftBrace,
            "Expected '{' to begin impl body.",
        )?;
        let mut methods = Vec::new();
        let mut names = std::collections::HashSet::new();
        while !is_punctuation(self.tokens.get(self.cursor), V05Punctuation::RightBrace) {
            if self.cursor >= self.tokens.len() {
                return Err(self.error("Unterminated impl declaration."));
            }
            let method = self.parse_method(true)?;
            self.validate_receiver(&method)?;
            if !names.insert(method.name.clone()) {
                return Err(self.error("Duplicate impl method."));
            }
            methods.push(method);
        }
        self.cursor += 1;
        let end = self.tokens[self.cursor - 1].span.end;
        Ok(V05ImplDecl {
            generics,
            target,
            trait_name,
            methods,
            span: start..end,
        })
    }

    fn validate_receiver(&self, method: &V05Method) -> Result<(), V05ParseError> {
        if method
            .parameters
            .first()
            .is_none_or(|parameter| parameter.name != "self")
        {
            return Err(self.error("Instance methods require `self` as their first parameter."));
        }
        Ok(())
    }

    fn parse_method(&mut self, has_body: bool) -> Result<V05Method, V05ParseError> {
        let start = self
            .tokens
            .get(self.cursor)
            .ok_or_else(|| self.error("Expected method declaration."))?
            .span
            .start;
        // fn is not a dedicated v0.5 keyword yet; it is an identifier token.
        if !self
            .tokens
            .get(self.cursor)
            .is_some_and(|t| matches!(&t.kind, V05TokenKind::Identifier(name) if name == "fn"))
        {
            return Err(self.error("Expected 'fn' method declaration."));
        }
        self.cursor += 1;
        let name = self.expect_identifier("Expected method name.")?;
        self.expect_punctuation(V05Punctuation::LeftParen, "Expected '(' after method name.")?;
        let mut parameters = Vec::new();
        let mut names = std::collections::HashSet::new();
        if !is_punctuation(self.tokens.get(self.cursor), V05Punctuation::RightParen) {
            loop {
                let token = self
                    .tokens
                    .get(self.cursor)
                    .ok_or_else(|| self.error("Expected method parameter."))?;
                let pname = match &token.kind {
                    V05TokenKind::Identifier(name) => name.clone(),
                    _ => return Err(self.error("Expected parameter name.")),
                };
                if !names.insert(pname.clone()) {
                    return Err(self.error("Duplicate method parameter."));
                }
                self.cursor += 1;
                let ty = if pname == "self" {
                    if is_punctuation(self.tokens.get(self.cursor), V05Punctuation::Colon) {
                        return Err(self.error(
                            "The self receiver type is inferred and must not be annotated.",
                        ));
                    }
                    None
                } else {
                    self.expect_punctuation(
                        V05Punctuation::Colon,
                        "Expected ':' after parameter name.",
                    )?;
                    Some(parse_type_expr(self.tokens, &mut self.cursor)?)
                };
                let end = self
                    .tokens
                    .get(self.cursor.saturating_sub(1))
                    .map(|t| t.span.end)
                    .unwrap_or(token.span.end);
                parameters.push(V05MethodParameter {
                    name: pname,
                    ty,
                    span: token.span.start..end,
                });
                if is_punctuation(self.tokens.get(self.cursor), V05Punctuation::Comma) {
                    self.cursor += 1;
                    if is_punctuation(self.tokens.get(self.cursor), V05Punctuation::RightParen) {
                        break;
                    }
                } else {
                    break;
                }
            }
        }
        self.expect_punctuation(
            V05Punctuation::RightParen,
            "Expected ')' after method parameters.",
        )?;
        let return_type = if is_punctuation(self.tokens.get(self.cursor), V05Punctuation::Arrow) {
            self.cursor += 1;
            Some(parse_type_expr(self.tokens, &mut self.cursor)?)
        } else {
            None
        };
        let body_span = if has_body {
            self.expect_punctuation(V05Punctuation::LeftBrace, "Expected method body '{'.")?;
            let body_start = self.tokens[self.cursor - 1].span.start;
            let mut depth = 1usize;
            while self.cursor < self.tokens.len() && depth > 0 {
                match self.tokens[self.cursor].kind {
                    V05TokenKind::Punctuation(V05Punctuation::LeftBrace) => depth += 1,
                    V05TokenKind::Punctuation(V05Punctuation::RightBrace) => depth -= 1,
                    _ => {}
                }
                self.cursor += 1;
                if depth > 64 {
                    return Err(
                        self.error("Maximum method-body brace nesting depth (64) exceeded.")
                    );
                }
            }
            if depth != 0 {
                return Err(self.error("Unterminated method body."));
            }
            Some(body_start..self.tokens[self.cursor - 1].span.end)
        } else {
            self.expect_punctuation(
                V05Punctuation::Semicolon,
                "Trait method signatures must end with ';'.",
            )?;
            None
        };
        let end = self
            .tokens
            .get(self.cursor.saturating_sub(1))
            .map(|t| t.span.end)
            .unwrap_or(start);
        Ok(V05Method {
            name,
            parameters,
            return_type,
            body_span,
            span: start..end,
        })
    }
}

#[cfg(test)]
mod v05_trait_impl_parser_tests {
    use super::{lex_v05, parse_trait_and_impl_declarations, V05TraitOrImpl};

    #[test]
    fn parses_trait_signatures_and_generic_inherent_impl_methods() {
        let tokens = lex_v05("trait Display { fn display(self) -> string; } impl<T: Display> Box<T> { fn get(self) -> T { return self.value; } }").unwrap();
        let declarations = parse_trait_and_impl_declarations(&tokens).unwrap();
        assert_eq!(declarations.len(), 2);
        match &declarations[0] {
            V05TraitOrImpl::Trait(decl) => {
                assert_eq!(decl.name, "Display");
                assert_eq!(decl.methods[0].name, "display");
                assert_eq!(decl.methods[0].parameters[0].name, "self");
                assert_eq!(
                    decl.methods[0].return_type.as_ref().unwrap(),
                    &super::TypeExpr::Named {
                        name: "string".into(),
                        arguments: vec![],
                        span: 36..42,
                    }
                );
                assert!(decl.methods[0].body_span.is_none());
            }
            _ => panic!("expected trait"),
        }
        match &declarations[1] {
            V05TraitOrImpl::Impl(decl) => {
                assert_eq!(decl.generics[0].parameter, "T");
                assert_eq!(
                    decl.target,
                    super::TypeExpr::Named {
                        name: "Box".into(),
                        arguments: vec![super::TypeExpr::Named {
                            name: "T".into(),
                            arguments: vec![],
                            span: 67..68,
                        }],
                        span: 63..69,
                    }
                );
                assert_eq!(decl.methods[0].name, "get");
                assert!(decl.methods[0].body_span.is_some());
            }
            _ => panic!("expected impl"),
        }
    }

    #[test]
    fn parses_trait_impl_and_captures_body_without_parsing_expressions() {
        let tokens = lex_v05("trait Display { fn display(self) -> string; } impl Display for Point { fn display(self) -> string { return \"p\"; } }").unwrap();
        let declarations = parse_trait_and_impl_declarations(&tokens).unwrap();
        match &declarations[1] {
            V05TraitOrImpl::Impl(decl) => {
                assert_eq!(decl.trait_name.as_deref(), Some("Display"));
                assert_eq!(
                    decl.target,
                    super::TypeExpr::Named {
                        name: "Point".into(),
                        arguments: vec![],
                        span: 63..68,
                    }
                );
            }
            _ => panic!("expected trait impl"),
        }
    }

    #[test]
    fn rejects_duplicate_method_names_and_parameter_names() {
        let duplicate_method = lex_v05("trait T { fn x(self); fn x(self); }").unwrap();
        assert!(parse_trait_and_impl_declarations(&duplicate_method).is_err());
        let duplicate_parameter =
            lex_v05("impl T { fn x(self, value: number, value: string) {} }").unwrap();
        assert!(parse_trait_and_impl_declarations(&duplicate_parameter).is_err());
    }

    #[test]
    fn rejects_missing_receiver_and_annotated_receiver() {
        let no_receiver = lex_v05("trait T { fn x(value: number); }").unwrap();
        assert!(parse_trait_and_impl_declarations(&no_receiver).is_err());
        let annotated = lex_v05("trait T { fn x(self: T); }").unwrap();
        assert!(parse_trait_and_impl_declarations(&annotated).is_err());
    }

    #[test]
    fn body_capture_is_bounded_and_requires_closing_brace() {
        let unterminated = lex_v05("impl T { fn x(self) { if (true) { }").unwrap();
        assert!(parse_trait_and_impl_declarations(&unterminated).is_err());
        let excessive =
            lex_v05(&format!("impl T {{ fn x(self) {{ {} }} }}", "{".repeat(70))).unwrap();
        assert!(parse_trait_and_impl_declarations(&excessive).is_err());
    }
}

// ---- Generic function declaration syntax (staged M1.1 slice) --------------

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct V05FunctionParameter {
    pub name: String,
    pub ty: TypeExpr,
    pub span: Range<usize>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct V05FunctionDecl {
    pub name: String,
    pub generics: Vec<GenericBound>,
    pub parameters: Vec<V05FunctionParameter>,
    pub return_type: Option<TypeExpr>,
    pub body_span: Range<usize>,
    pub span: Range<usize>,
}

/// Parses generic and nongeneric function declarations while preserving their
/// body token ranges. Expression and statement parsing is a separate stage.
pub fn parse_function_declarations(
    tokens: &[V05Token],
) -> Result<Vec<V05FunctionDecl>, V05ParseError> {
    let mut parser = V05FunctionParser { tokens, cursor: 0 };
    let mut functions = Vec::new();
    let mut names = std::collections::HashSet::new();
    while parser.cursor < tokens.len() {
        let function = parser.parse_function()?;
        if !names.insert(function.name.clone()) {
            return Err(parser.error("Duplicate function declaration."));
        }
        functions.push(function);
    }
    Ok(functions)
}

struct V05FunctionParser<'a> {
    tokens: &'a [V05Token],
    cursor: usize,
}

impl V05FunctionParser<'_> {
    fn error(&self, message: &str) -> V05ParseError {
        parse_error(self.tokens, self.cursor, message)
    }

    fn expect_punctuation(
        &mut self,
        punctuation: V05Punctuation,
        message: &str,
    ) -> Result<(), V05ParseError> {
        if is_punctuation(self.tokens.get(self.cursor), punctuation) {
            self.cursor += 1;
            Ok(())
        } else {
            Err(self.error(message))
        }
    }

    fn parse_function(&mut self) -> Result<V05FunctionDecl, V05ParseError> {
        let start_token = self
            .tokens
            .get(self.cursor)
            .ok_or_else(|| self.error("Expected function declaration."))?;
        if !matches!(&start_token.kind, V05TokenKind::Identifier(keyword) if keyword == "fn") {
            return Err(self.error("Expected function declaration keyword."));
        }
        let start = start_token.span.start;
        self.cursor += 1;
        let name_token = self
            .tokens
            .get(self.cursor)
            .ok_or_else(|| self.error("Expected function name."))?;
        let name = match &name_token.kind {
            V05TokenKind::Identifier(name) => name.clone(),
            _ => return Err(self.error("Expected function name.")),
        };
        self.cursor += 1;
        let generics = parse_generic_parameters(self.tokens, &mut self.cursor)?;
        self.expect_punctuation(
            V05Punctuation::LeftParen,
            "Expected '(' after function name/generics.",
        )?;
        let mut parameters = Vec::new();
        let mut names = std::collections::HashSet::new();
        while !is_punctuation(self.tokens.get(self.cursor), V05Punctuation::RightParen) {
            let token = self
                .tokens
                .get(self.cursor)
                .ok_or_else(|| self.error("Expected function parameter."))?;
            let parameter_name = match &token.kind {
                V05TokenKind::Identifier(name) => name.clone(),
                _ => return Err(self.error("Expected parameter name.")),
            };
            if !names.insert(parameter_name.clone()) {
                return Err(self.error("Duplicate function parameter."));
            }
            self.cursor += 1;
            self.expect_punctuation(V05Punctuation::Colon, "Expected ':' after parameter name.")?;
            let ty = parse_type_expr(self.tokens, &mut self.cursor)?;
            let end = self
                .tokens
                .get(self.cursor.saturating_sub(1))
                .map(|t| t.span.end)
                .unwrap_or(token.span.end);
            parameters.push(V05FunctionParameter {
                name: parameter_name,
                ty,
                span: token.span.start..end,
            });
            if is_punctuation(self.tokens.get(self.cursor), V05Punctuation::Comma) {
                self.cursor += 1;
                if is_punctuation(self.tokens.get(self.cursor), V05Punctuation::RightParen) {
                    break;
                }
            } else {
                break;
            }
        }
        self.expect_punctuation(V05Punctuation::RightParen, "Expected ')' after parameters.")?;
        let return_type = if is_punctuation(self.tokens.get(self.cursor), V05Punctuation::Arrow) {
            self.cursor += 1;
            Some(parse_type_expr(self.tokens, &mut self.cursor)?)
        } else {
            None
        };
        self.expect_punctuation(V05Punctuation::LeftBrace, "Expected function body.")?;
        let body_start = self.tokens[self.cursor - 1].span.start;
        let mut depth = 1usize;
        while self.cursor < self.tokens.len() && depth > 0 {
            match self.tokens[self.cursor].kind {
                V05TokenKind::Punctuation(V05Punctuation::LeftBrace) => depth += 1,
                V05TokenKind::Punctuation(V05Punctuation::RightBrace) => depth -= 1,
                _ => {}
            }
            self.cursor += 1;
            if depth > 64 {
                return Err(self.error("Maximum function-body brace nesting depth (64) exceeded."));
            }
        }
        if depth != 0 {
            return Err(self.error("Unterminated function body."));
        }
        let body_span = body_start..self.tokens[self.cursor - 1].span.end;
        let end = self.tokens[self.cursor - 1].span.end;
        Ok(V05FunctionDecl {
            name,
            generics,
            parameters,
            return_type,
            body_span,
            span: start..end,
        })
    }
}

#[cfg(test)]
mod v05_function_parser_tests {
    use super::{lex_v05, parse_function_declarations};

    #[test]
    fn parses_generic_function_signature_and_captures_body() {
        let tokens = lex_v05("fn identity<T>(value: T) -> T { return value; } fn sum(a: number, b: number) { return a + b; }").unwrap();
        let functions = parse_function_declarations(&tokens).unwrap();
        assert_eq!(functions.len(), 2);
        assert_eq!(functions[0].name, "identity");
        assert_eq!(functions[0].generics[0].parameter, "T");
        assert_eq!(functions[0].parameters[0].name, "value");
        assert!(functions[0].return_type.is_some());
        assert_eq!(functions[1].name, "sum");
        assert_eq!(functions[1].parameters.len(), 2);
        assert!(functions[1].return_type.is_none());
    }

    #[test]
    fn rejects_duplicate_function_and_parameter_names() {
        let duplicate_functions = lex_v05("fn f() {} fn f() {}").unwrap();
        assert!(parse_function_declarations(&duplicate_functions).is_err());
        let duplicate_parameters = lex_v05("fn f(a: number, a: string) {}").unwrap();
        assert!(parse_function_declarations(&duplicate_parameters).is_err());
    }

    #[test]
    fn rejects_unterminated_and_deep_function_bodies() {
        let unterminated = lex_v05("fn f() { if (true) { }").unwrap();
        assert!(parse_function_declarations(&unterminated).is_err());
        let deep = lex_v05(&format!("fn f() {{ {} }}", "{".repeat(70))).unwrap();
        assert!(parse_function_declarations(&deep).is_err());
    }
}

// ---- Struct literal field syntax (staged M1.1 slice) ----------------------

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct V05StructLiteralField {
    pub name: String,
    pub value_span: Range<usize>,
    pub span: Range<usize>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct V05StructLiteral {
    pub ty: TypeExpr,
    pub fields: Vec<V05StructLiteralField>,
    pub span: Range<usize>,
}

/// Parses a standalone nominal struct literal. Field expressions are retained
/// as source spans and will be parsed by the expression parser in integration.
pub fn parse_struct_literal(
    tokens: &[V05Token],
    cursor: &mut usize,
) -> Result<V05StructLiteral, V05ParseError> {
    let original = *cursor;
    match parse_struct_literal_at(tokens, cursor) {
        Ok(literal) => Ok(literal),
        Err(error) => {
            *cursor = original;
            Err(error)
        }
    }
}

fn parse_struct_literal_at(
    tokens: &[V05Token],
    cursor: &mut usize,
) -> Result<V05StructLiteral, V05ParseError> {
    let ty = parse_type_expr(tokens, cursor)?;
    let start = ty_span_start(&ty);
    if !is_punctuation(tokens.get(*cursor), V05Punctuation::LeftBrace) {
        return Err(parse_error(
            tokens,
            *cursor,
            "Expected '{' after struct literal type.",
        ));
    }
    *cursor += 1;
    let mut fields = Vec::new();
    let mut names = std::collections::HashSet::new();
    while !is_punctuation(tokens.get(*cursor), V05Punctuation::RightBrace) {
        if *cursor >= tokens.len() {
            return Err(parse_error(tokens, *cursor, "Unterminated struct literal."));
        }
        let field_token = tokens
            .get(*cursor)
            .ok_or_else(|| parse_error(tokens, *cursor, "Expected field name."))?;
        let field_name = match &field_token.kind {
            V05TokenKind::Identifier(name) => name.clone(),
            _ => {
                return Err(parse_error(
                    tokens,
                    *cursor,
                    "Expected struct literal field name.",
                ))
            }
        };
        if !names.insert(field_name.clone()) {
            return Err(parse_error(
                tokens,
                *cursor,
                "Duplicate struct literal field.",
            ));
        }
        *cursor += 1;
        if !is_punctuation(tokens.get(*cursor), V05Punctuation::Colon) {
            return Err(parse_error(
                tokens,
                *cursor,
                "Expected ':' after field name.",
            ));
        }
        *cursor += 1;
        let value_start_token = tokens
            .get(*cursor)
            .ok_or_else(|| parse_error(tokens, *cursor, "Expected field expression."))?;
        let value_start = value_start_token.span.start;
        let mut nested = 0usize;
        let mut value_end = value_start;
        while *cursor < tokens.len() {
            let token = &tokens[*cursor];
            match token.kind {
                V05TokenKind::Punctuation(
                    V05Punctuation::LeftParen
                    | V05Punctuation::LeftBracket
                    | V05Punctuation::LeftBrace,
                ) => nested += 1,
                V05TokenKind::Punctuation(
                    V05Punctuation::RightParen | V05Punctuation::RightBracket,
                ) => {
                    if nested == 0 {
                        return Err(parse_error(
                            tokens,
                            *cursor,
                            "Unbalanced field expression delimiter.",
                        ));
                    }
                    nested -= 1;
                }
                V05TokenKind::Punctuation(V05Punctuation::RightBrace) if nested == 0 => break,
                V05TokenKind::Punctuation(V05Punctuation::RightBrace) => nested -= 1,
                V05TokenKind::Punctuation(V05Punctuation::Comma) if nested == 0 => break,
                _ => {}
            }
            value_end = token.span.end;
            *cursor += 1;
            if nested > 64 {
                return Err(parse_error(
                    tokens,
                    *cursor,
                    "Maximum struct-literal expression nesting (64) exceeded.",
                ));
            }
        }
        if value_end == value_start {
            return Err(parse_error(
                tokens,
                *cursor,
                "Field expression cannot be empty.",
            ));
        }
        fields.push(V05StructLiteralField {
            name: field_name,
            value_span: value_start..value_end,
            span: field_token.span.start..value_end,
        });
        if is_punctuation(tokens.get(*cursor), V05Punctuation::Comma) {
            *cursor += 1;
            if is_punctuation(tokens.get(*cursor), V05Punctuation::RightBrace) {
                break;
            }
        } else {
            break;
        }
    }
    if !is_punctuation(tokens.get(*cursor), V05Punctuation::RightBrace) {
        return Err(parse_error(
            tokens,
            *cursor,
            "Expected '}' to close struct literal.",
        ));
    }
    *cursor += 1;
    let end = tokens[*cursor - 1].span.end;
    Ok(V05StructLiteral {
        ty,
        fields,
        span: start..end,
    })
}

fn ty_span_start(ty: &TypeExpr) -> usize {
    match ty {
        TypeExpr::Named { span, .. } | TypeExpr::Any { span } => span.start,
    }
}

#[cfg(test)]
mod v05_struct_literal_parser_tests {
    use super::{lex_v05, parse_struct_literal};

    #[test]
    fn parses_struct_literal_fields_and_generic_type() {
        let tokens = lex_v05("Point { x: 1 + 2, y: [3, 4], }").unwrap();
        let mut cursor = 0;
        let literal = parse_struct_literal(&tokens, &mut cursor).unwrap();
        assert_eq!(literal.fields.len(), 2);
        assert_eq!(literal.fields[0].name, "x");
        assert_eq!(literal.fields[1].name, "y");
        assert_eq!(cursor, tokens.len());

        let tokens = lex_v05("Box<number> { value: 42 }").unwrap();
        let mut cursor = 0;
        let literal = parse_struct_literal(&tokens, &mut cursor).unwrap();
        assert_eq!(literal.fields[0].name, "value");
    }

    #[test]
    fn rejects_duplicate_and_empty_struct_literal_fields() {
        let duplicate = lex_v05("Point { x: 1, x: 2 }").unwrap();
        assert!(parse_struct_literal(&duplicate, &mut 0).is_err());
        let empty = lex_v05("Point { x: }").unwrap();
        assert!(parse_struct_literal(&empty, &mut 0).is_err());
    }

    #[test]
    fn struct_literal_parser_rolls_back_cursor_on_failure() {
        let tokens = lex_v05("Point { x: }").unwrap();
        let mut cursor = 0;
        assert!(parse_struct_literal(&tokens, &mut cursor).is_err());
        assert_eq!(cursor, 0);
    }
}

// ---- Enum constructor syntax (staged M1.1 slice) --------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct V05EnumConstructor {
    pub enum_type: TypeExpr,
    pub variant: String,
    pub payload_spans: Vec<Range<usize>>,
    pub span: Range<usize>,
}

/// Parses a fully-qualified enum constructor and retains payload expression
/// spans; payload expressions and expected-arity/type checks remain later phases.
pub fn parse_enum_constructor(
    tokens: &[V05Token],
    cursor: &mut usize,
) -> Result<V05EnumConstructor, V05ParseError> {
    let original = *cursor;
    match parse_enum_constructor_at(tokens, cursor) {
        Ok(value) => Ok(value),
        Err(error) => {
            *cursor = original;
            Err(error)
        }
    }
}

fn parse_enum_constructor_at(
    tokens: &[V05Token],
    cursor: &mut usize,
) -> Result<V05EnumConstructor, V05ParseError> {
    let enum_type = parse_type_expr(tokens, cursor)?;
    let start = ty_span_start(&enum_type);
    if !is_punctuation(tokens.get(*cursor), V05Punctuation::DoubleColon) {
        return Err(parse_error(
            tokens,
            *cursor,
            "Expected '::' before enum variant.",
        ));
    }
    *cursor += 1;
    let variant_token = tokens
        .get(*cursor)
        .ok_or_else(|| parse_error(tokens, *cursor, "Expected enum variant name."))?;
    let variant = match &variant_token.kind {
        V05TokenKind::Identifier(name) => name.clone(),
        _ => return Err(parse_error(tokens, *cursor, "Expected enum variant name.")),
    };
    *cursor += 1;
    let mut payload_spans = Vec::new();
    if is_punctuation(tokens.get(*cursor), V05Punctuation::LeftParen) {
        *cursor += 1;
        while !is_punctuation(tokens.get(*cursor), V05Punctuation::RightParen) {
            let value_token = tokens.get(*cursor).ok_or_else(|| {
                parse_error(tokens, *cursor, "Unterminated enum constructor payload.")
            })?;
            let value_start = value_token.span.start;
            let mut depth = 0usize;
            let mut value_end = value_start;
            while *cursor < tokens.len() {
                let token = &tokens[*cursor];
                match token.kind {
                    V05TokenKind::Punctuation(
                        V05Punctuation::LeftParen
                        | V05Punctuation::LeftBracket
                        | V05Punctuation::LeftBrace,
                    ) => depth += 1,
                    V05TokenKind::Punctuation(V05Punctuation::RightParen) if depth == 0 => break,
                    V05TokenKind::Punctuation(
                        V05Punctuation::RightParen
                        | V05Punctuation::RightBracket
                        | V05Punctuation::RightBrace,
                    ) => {
                        if depth == 0 {
                            return Err(parse_error(
                                tokens,
                                *cursor,
                                "Unbalanced enum payload delimiter.",
                            ));
                        }
                        depth -= 1;
                    }
                    V05TokenKind::Punctuation(V05Punctuation::Comma) if depth == 0 => break,
                    _ => {}
                }
                value_end = token.span.end;
                *cursor += 1;
                if depth > 64 {
                    return Err(parse_error(
                        tokens,
                        *cursor,
                        "Maximum enum payload nesting (64) exceeded.",
                    ));
                }
            }
            if value_end == value_start {
                return Err(parse_error(
                    tokens,
                    *cursor,
                    "Enum payload expression cannot be empty.",
                ));
            }
            payload_spans.push(value_start..value_end);
            if is_punctuation(tokens.get(*cursor), V05Punctuation::Comma) {
                *cursor += 1;
                if is_punctuation(tokens.get(*cursor), V05Punctuation::RightParen) {
                    break;
                }
            } else {
                break;
            }
        }
        if !is_punctuation(tokens.get(*cursor), V05Punctuation::RightParen) {
            return Err(parse_error(
                tokens,
                *cursor,
                "Expected ')' to close enum constructor.",
            ));
        }
        *cursor += 1;
    }
    let end = tokens
        .get(cursor.saturating_sub(1))
        .map(|t| t.span.end)
        .unwrap_or(variant_token.span.end);
    Ok(V05EnumConstructor {
        enum_type,
        variant,
        payload_spans,
        span: start..end,
    })
}

#[cfg(test)]
mod v05_enum_constructor_parser_tests {
    use super::{lex_v05, parse_enum_constructor};

    #[test]
    fn parses_unit_and_tuple_enum_constructors() {
        let tokens = lex_v05("Direction::North").unwrap();
        let mut cursor = 0;
        let value = parse_enum_constructor(&tokens, &mut cursor).unwrap();
        assert_eq!(value.variant, "North");
        assert!(value.payload_spans.is_empty());

        let tokens = lex_v05("Result<number, string>::Ok(1, [2, 3],)").unwrap();
        let mut cursor = 0;
        let value = parse_enum_constructor(&tokens, &mut cursor).unwrap();
        assert_eq!(value.variant, "Ok");
        assert_eq!(value.payload_spans.len(), 2);
        assert_eq!(cursor, tokens.len());
    }

    #[test]
    fn rejects_empty_and_unterminated_enum_payloads() {
        let empty = lex_v05("Result::Ok(,)").unwrap();
        assert!(parse_enum_constructor(&empty, &mut 0).is_err());
        let missing_close = lex_v05("Result::Ok(1").unwrap();
        assert!(parse_enum_constructor(&missing_close, &mut 0).is_err());
    }

    #[test]
    fn enum_constructor_parse_failure_rolls_back_cursor() {
        let tokens = lex_v05("Result::Ok(, 1)").unwrap();
        let mut cursor = 0;
        assert!(parse_enum_constructor(&tokens, &mut cursor).is_err());
        assert_eq!(cursor, 0);
    }
}

// ---- Statement-level match syntax (staged M1.1 slice) ---------------------

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum V05MatchPattern {
    Wildcard {
        span: Range<usize>,
    },
    Variant {
        enum_type: TypeExpr,
        variant: String,
        bindings: Vec<String>,
        span: Range<usize>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct V05MatchArm {
    pub pattern: V05MatchPattern,
    pub body_span: Range<usize>,
    pub span: Range<usize>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct V05MatchStatement {
    pub scrutinee_span: Range<usize>,
    pub arms: Vec<V05MatchArm>,
    pub span: Range<usize>,
}

/// Parses statement-level match arms and captures scrutinee/arm-block spans.
/// Exhaustiveness and pattern/type validation are intentionally semantic stages.
pub fn parse_match_statement(
    tokens: &[V05Token],
    cursor: &mut usize,
) -> Result<V05MatchStatement, V05ParseError> {
    let original = *cursor;
    match parse_match_statement_at(tokens, cursor) {
        Ok(statement) => Ok(statement),
        Err(error) => {
            *cursor = original;
            Err(error)
        }
    }
}

fn parse_match_statement_at(
    tokens: &[V05Token],
    cursor: &mut usize,
) -> Result<V05MatchStatement, V05ParseError> {
    let start_token = tokens
        .get(*cursor)
        .ok_or_else(|| parse_error(tokens, *cursor, "Expected match statement."))?;
    if !matches!(&start_token.kind, V05TokenKind::Keyword(V05Keyword::Match)) {
        return Err(parse_error(tokens, *cursor, "Expected match keyword."));
    }
    let start = start_token.span.start;
    *cursor += 1;
    let scrutinee_start_token = tokens
        .get(*cursor)
        .ok_or_else(|| parse_error(tokens, *cursor, "Expected match scrutinee."))?;
    let scrutinee_start = scrutinee_start_token.span.start;
    let mut depth = 0usize;
    let mut scrutinee_end = scrutinee_start;
    while *cursor < tokens.len() {
        let token = &tokens[*cursor];
        match token.kind {
            V05TokenKind::Punctuation(V05Punctuation::LeftParen | V05Punctuation::LeftBracket) => {
                depth += 1
            }
            V05TokenKind::Punctuation(
                V05Punctuation::RightParen | V05Punctuation::RightBracket,
            ) => {
                if depth == 0 {
                    return Err(parse_error(
                        tokens,
                        *cursor,
                        "Unbalanced match scrutinee delimiter.",
                    ));
                }
                depth -= 1;
            }
            V05TokenKind::Punctuation(V05Punctuation::LeftBrace) if depth == 0 => break,
            V05TokenKind::Punctuation(V05Punctuation::LeftBrace) => depth += 1,
            V05TokenKind::Punctuation(V05Punctuation::RightBrace) => {
                if depth == 0 {
                    return Err(parse_error(
                        tokens,
                        *cursor,
                        "Expected match arm block after scrutinee.",
                    ));
                }
                depth -= 1;
            }
            _ => {}
        }
        scrutinee_end = token.span.end;
        *cursor += 1;
        if depth > 64 {
            return Err(parse_error(
                tokens,
                *cursor,
                "Maximum match scrutinee nesting (64) exceeded.",
            ));
        }
    }
    if scrutinee_end == scrutinee_start {
        return Err(parse_error(
            tokens,
            *cursor,
            "Match scrutinee cannot be empty.",
        ));
    }
    if !is_punctuation(tokens.get(*cursor), V05Punctuation::LeftBrace) {
        return Err(parse_error(
            tokens,
            *cursor,
            "Expected '{' before match arms.",
        ));
    }
    *cursor += 1;
    let mut arms = Vec::new();
    let mut variant_keys = std::collections::HashSet::new();
    let mut wildcard_seen = false;
    while !is_punctuation(tokens.get(*cursor), V05Punctuation::RightBrace) {
        if *cursor >= tokens.len() {
            return Err(parse_error(
                tokens,
                *cursor,
                "Unterminated match statement.",
            ));
        }
        let arm_start = tokens[*cursor].span.start;
        let pattern = if is_punctuation(tokens.get(*cursor), V05Punctuation::Underscore) {
            if wildcard_seen {
                return Err(parse_error(
                    tokens,
                    *cursor,
                    "Duplicate wildcard match arm.",
                ));
            }
            wildcard_seen = true;
            let span = tokens[*cursor].span.clone();
            *cursor += 1;
            V05MatchPattern::Wildcard { span }
        } else {
            let enum_type = parse_type_expr(tokens, cursor)?;
            if !is_punctuation(tokens.get(*cursor), V05Punctuation::DoubleColon) {
                return Err(parse_error(
                    tokens,
                    *cursor,
                    "Match variants must be fully qualified with '::'.",
                ));
            }
            *cursor += 1;
            let variant_token = tokens.get(*cursor).ok_or_else(|| {
                parse_error(tokens, *cursor, "Expected variant name in match pattern.")
            })?;
            let variant = match &variant_token.kind {
                V05TokenKind::Identifier(name) => name.clone(),
                _ => {
                    return Err(parse_error(
                        tokens,
                        *cursor,
                        "Expected variant name in match pattern.",
                    ))
                }
            };
            *cursor += 1;
            let mut bindings = Vec::new();
            if is_punctuation(tokens.get(*cursor), V05Punctuation::LeftParen) {
                *cursor += 1;
                let mut names = std::collections::HashSet::new();
                while !is_punctuation(tokens.get(*cursor), V05Punctuation::RightParen) {
                    let binder = tokens.get(*cursor).ok_or_else(|| {
                        parse_error(tokens, *cursor, "Unterminated match pattern bindings.")
                    })?;
                    let name = match &binder.kind {
                        V05TokenKind::Identifier(name) => name.clone(),
                        _ => {
                            return Err(parse_error(
                                tokens,
                                *cursor,
                                "Expected pattern binding name.",
                            ))
                        }
                    };
                    if !names.insert(name.clone()) {
                        return Err(parse_error(
                            tokens,
                            *cursor,
                            "Duplicate match pattern binding.",
                        ));
                    }
                    bindings.push(name);
                    *cursor += 1;
                    if is_punctuation(tokens.get(*cursor), V05Punctuation::Comma) {
                        *cursor += 1;
                        if is_punctuation(tokens.get(*cursor), V05Punctuation::RightParen) {
                            break;
                        }
                    } else {
                        break;
                    }
                }
                if !is_punctuation(tokens.get(*cursor), V05Punctuation::RightParen) {
                    return Err(parse_error(
                        tokens,
                        *cursor,
                        "Expected ')' after pattern bindings.",
                    ));
                }
                *cursor += 1;
            }
            let variant_key = (type_expr_name(&enum_type).to_owned(), variant.clone());
            if !variant_keys.insert(variant_key) {
                return Err(parse_error(tokens, *cursor, "Duplicate match variant arm."));
            }
            let pattern_end = tokens
                .get(cursor.saturating_sub(1))
                .map(|t| t.span.end)
                .unwrap_or(variant_token.span.end);
            V05MatchPattern::Variant {
                enum_type,
                variant,
                bindings,
                span: arm_start..pattern_end,
            }
        };
        if wildcard_seen && !matches!(pattern, V05MatchPattern::Wildcard { .. }) {
            return Err(parse_error(
                tokens,
                *cursor,
                "Wildcard match arm must be last.",
            ));
        }
        // The arrow is lexed as '=' followed by '>' for this staged lexer.
        if !is_punctuation(tokens.get(*cursor), V05Punctuation::Equal)
            || !is_punctuation(tokens.get(*cursor + 1), V05Punctuation::Greater)
        {
            return Err(parse_error(
                tokens,
                *cursor,
                "Expected '=>' after match pattern.",
            ));
        }
        *cursor += 2;
        if !is_punctuation(tokens.get(*cursor), V05Punctuation::LeftBrace) {
            return Err(parse_error(
                tokens,
                *cursor,
                "Match arm body must be a block.",
            ));
        }
        let body_start = tokens[*cursor].span.start;
        let mut body_depth = 0usize;
        while *cursor < tokens.len() {
            match tokens[*cursor].kind {
                V05TokenKind::Punctuation(V05Punctuation::LeftBrace) => body_depth += 1,
                V05TokenKind::Punctuation(V05Punctuation::RightBrace) => body_depth -= 1,
                _ => {}
            }
            *cursor += 1;
            if body_depth == 0 {
                break;
            }
            if body_depth > 64 {
                return Err(parse_error(
                    tokens,
                    *cursor,
                    "Maximum match-arm block nesting (64) exceeded.",
                ));
            }
        }
        if body_depth != 0 {
            return Err(parse_error(
                tokens,
                *cursor,
                "Unterminated match arm block.",
            ));
        }
        let body_end = tokens[*cursor - 1].span.end;
        let arm_end = body_end;
        arms.push(V05MatchArm {
            pattern,
            body_span: body_start..body_end,
            span: arm_start..arm_end,
        });
        if is_punctuation(tokens.get(*cursor), V05Punctuation::Comma) {
            *cursor += 1;
        }
    }
    if arms.is_empty() {
        return Err(parse_error(
            tokens,
            *cursor,
            "Match statement requires at least one arm.",
        ));
    }
    *cursor += 1;
    let end = tokens[*cursor - 1].span.end;
    Ok(V05MatchStatement {
        scrutinee_span: scrutinee_start..scrutinee_end,
        arms,
        span: start..end,
    })
}

fn type_expr_name(ty: &TypeExpr) -> &str {
    match ty {
        TypeExpr::Named { name, .. } => name,
        TypeExpr::Any { .. } => "any",
    }
}

#[cfg(test)]
mod v05_match_parser_tests {
    use super::{lex_v05, parse_match_statement, V05MatchPattern};

    #[test]
    fn parses_qualified_variant_patterns_and_wildcard() {
        let tokens = lex_v05("match value { Result<number, string>::Ok(x) => { print(x); } Result<number, string>::Err(message) => { print(message); } _ => { } }").unwrap();
        let mut cursor = 0;
        let statement = parse_match_statement(&tokens, &mut cursor).unwrap();
        assert_eq!(statement.arms.len(), 3);
        assert!(
            matches!(statement.arms[0].pattern, V05MatchPattern::Variant { ref bindings, .. } if bindings == &["x"])
        );
        assert!(matches!(
            statement.arms[2].pattern,
            V05MatchPattern::Wildcard { .. }
        ));
        assert_eq!(cursor, tokens.len());
    }

    #[test]
    fn rejects_duplicate_variants_duplicate_binders_and_nonfinal_wildcard() {
        let duplicate_variant = lex_v05("match value { E::A => {} E::A => {} }").unwrap();
        assert!(parse_match_statement(&duplicate_variant, &mut 0).is_err());
        let duplicate_binder = lex_v05("match value { E::A(x, x) => {} }").unwrap();
        assert!(parse_match_statement(&duplicate_binder, &mut 0).is_err());
        let wildcard_not_last = lex_v05("match value { _ => {} E::A => {} }").unwrap();
        assert!(parse_match_statement(&wildcard_not_last, &mut 0).is_err());
    }

    #[test]
    fn match_parser_rolls_back_on_malformed_arm() {
        let tokens = lex_v05("match value { E::A =>").unwrap();
        let mut cursor = 0;
        assert!(parse_match_statement(&tokens, &mut cursor).is_err());
        assert_eq!(cursor, 0);
    }
}
