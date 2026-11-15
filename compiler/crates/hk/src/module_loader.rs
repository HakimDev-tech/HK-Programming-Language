use crate::{ast::S, runtime, HkError, Result};
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
};

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
    total_source_bytes: &mut usize,
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
    const MAX_MODULE_SOURCE_BYTES: u64 = 1_048_576;
    const MAX_MODULE_GRAPH_SOURCE_BYTES: usize = 16 * 1_048_576;
    let metadata = fs::metadata(&canonical).map_err(|error| {
        module_error(
            "E4006",
            format!("Cannot inspect module '{}': {error}.", canonical.display()),
            &canonical.display().to_string(),
            1,
            1,
            "",
        )
    })?;
    if !metadata.is_file() || metadata.len() > MAX_MODULE_SOURCE_BYTES {
        return Err(module_error(
            "E4006",
            "Module source must be a regular file no larger than 1 MiB.",
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
    if source.len() as u64 > MAX_MODULE_SOURCE_BYTES {
        return Err(module_error(
            "E4006",
            "Module source exceeds the 1 MiB per-file limit.",
            &canonical.display().to_string(),
            1,
            1,
            &source,
        ));
    }
    if total_source_bytes.saturating_add(source.len()) > MAX_MODULE_GRAPH_SOURCE_BYTES {
        return Err(module_error(
            "E4006",
            "Module graph source exceeds the 16 MiB aggregate limit.",
            &canonical.display().to_string(),
            1,
            1,
            &source,
        ));
    }
    *total_source_bytes += source.len();
    let source_name = canonical.display().to_string();
    let tokens = crate::lexer::Lexer::new(&source_name, &source).scan()?;
    let statements = crate::parser::Parser::new(tokens, &source_name, &source).parse()?;

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
            load_module_graph(&target, modules, order, visiting, total_source_bytes)?;
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
    let mut total_source_bytes = 0usize;
    let _entry = load_module_graph(
        entry,
        &mut modules,
        &mut order,
        &mut visiting,
        &mut total_source_bytes,
    )?;

    // Validate the complete top-level module namespace before execution.
    for path in &order {
        let module = modules.get(path).expect("loaded module must exist");
        let mut local_names = HashSet::new();
        local_names.insert("print".to_owned());
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
            crate::type_checker::TypeChecker::new(
                &module.path.display().to_string(),
                &module.source,
            )
            .check(&module.statements)?;
        }
    }

    // Allocate every environment before linking imports. Strong references in this
    // map keep defining module environments alive for imported functions.
    let mut environments = HashMap::<PathBuf, runtime::ModuleEnv>::new();
    for path in &order {
        environments.insert(path.clone(), runtime::new_module_env());
    }
    let std_math = runtime::new_std_math_env();

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
                    runtime::link_module_import(
                        environment,
                        &target,
                        &name.local,
                        &name.imported,
                    )
                    .map_err(|message| {
                        module_error(
                            "E4003",
                            message,
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
        output.extend(runtime::execute_module(
            &module.path.display().to_string(),
            &module.source,
            &module.statements,
            environment,
        )?);
    }
    Ok(output)
}

