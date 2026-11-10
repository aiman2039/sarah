use std::cmp::Reverse;
use std::path::Path;

use super::model::TopFn;
use super::warning::{Warning, WarningKind};
use crate::config::{LimitRules, Limits};
use crate::count::FileStats;
use crate::functions::TypeStats;
use crate::structure::FolderRules;

impl Warning {
    /// Files over their token, line or function-count limit, in report order.
    pub fn for_files(files: &[FileStats], rules: &LimitRules) -> Vec<Self> {
        files
            .iter()
            .flat_map(|f| {
                let limits = rules.for_path(&f.path);
                [Self::for_file(f, &limits), Self::for_file_fns(f, &limits)]
                    .into_iter()
                    .flatten()
            })
            .collect()
    }

    /// Structs and classes over their field or method limits.
    pub fn for_types(files: &[FileStats], rules: &LimitRules) -> Vec<Self> {
        files
            .iter()
            .flat_map(|f| {
                let limits = rules.for_path(&f.path);
                f.types
                    .iter()
                    .filter_map(move |ty| Self::for_type(&f.path, ty, &limits))
            })
            .collect()
    }

    /// Imports matching `forbid_imports` in `[[files]]` or `[[folder]]`.
    pub fn for_imports(
        files: &[FileStats],
        rules: &LimitRules,
        folders: &FolderRules,
    ) -> Vec<Self> {
        files
            .iter()
            .flat_map(|f| Self::file_imports(f, rules, folders))
            .collect()
    }

    /// Functions over their size, nesting, parameter or complexity limits, in ranked order.
    pub fn for_functions(functions: &[TopFn], rules: &LimitRules) -> Vec<Self> {
        functions
            .iter()
            .flat_map(|func| {
                let limits = rules.for_path(&func.path);
                [
                    Self::for_large_function(func, &limits),
                    Self::for_deep_function(func, &limits),
                    Self::for_many_params(func, &limits),
                    Self::for_many_bool_params(func, &limits),
                    Self::for_complex_function(func, &limits),
                ]
            })
            .flatten()
            .collect()
    }

    /// Malformed or misplaced `sarah-allow` comments, file by file.
    pub fn for_allow_problems(files: &[FileStats]) -> Vec<Self> {
        files
            .iter()
            .flat_map(|f| {
                f.allows.problems.iter().map(|p| {
                    Self::new(
                        f.path.clone(),
                        WarningKind::BadAllow,
                        format!("sarah-allow at L{}: {}", p.line, p.message),
                    )
                    .at_line(p.line)
                })
            })
            .collect()
    }

    fn file_imports(f: &FileStats, rules: &LimitRules, folders: &FolderRules) -> Vec<Self> {
        let by_file = rules.forbid_imports_for(&f.path);
        let dir = match f.path.parent() {
            Some(p) if !p.as_os_str().is_empty() => p,
            _ => Path::new("."),
        };
        let policy = folders.policy_for(dir);
        let by_folder = policy.as_ref().and_then(|p| p.forbid_imports);
        f.imports
            .iter()
            .filter_map(|imp| {
                let pat = by_file
                    .and_then(|p| p.find(&imp.path))
                    .or_else(|| by_folder.and_then(|p| p.find(&imp.path)))?;
                Some(
                    Self::new(
                        f.path.clone(),
                        WarningKind::ForbiddenImport,
                        format!(
                            "import {:?} at L{} matches forbidden {pat:?}",
                            imp.path, imp.line
                        ),
                    )
                    .at_line(imp.line)
                    .about(imp.path.clone()),
                )
            })
            .collect()
    }

    fn for_file(f: &FileStats, limits: &Limits) -> Option<Self> {
        let mut over = Vec::new();
        if f.tokens > limits.file_tokens {
            over.push(format!("{} tokens (>{})", f.tokens, limits.file_tokens));
        }
        if f.lines > limits.file_lines {
            over.push(format!("{} lines (>{})", f.lines, limits.file_lines));
        }
        if over.is_empty() {
            return None;
        }
        let mut biggest: Vec<_> = f.functions.iter().flatten().collect();
        biggest.sort_by_key(|x| Reverse(x.lines));
        let hint = if biggest.is_empty() {
            String::new()
        } else {
            let names: Vec<_> = biggest
                .iter()
                .take(3)
                .map(|x| format!("{} ({} lines)", x.name, x.lines))
                .collect();
            format!("; largest fns: {}", names.join(", "))
        };
        Some(Self::new(
            f.path.clone(),
            WarningKind::LargeFile,
            format!("{} — consider splitting{hint}", over.join(", ")),
        ))
    }

    fn for_file_fns(f: &FileStats, limits: &Limits) -> Option<Self> {
        let max = limits.file_fns?;
        let count = f.functions.as_ref()?.len();
        (count > max.0).then(|| {
            Self::new(
                f.path.clone(),
                WarningKind::TooManyFunctions,
                format!("{count} functions (>{max}) — split into focused modules"),
            )
        })
    }

    fn for_type(path: &Path, ty: &TypeStats, limits: &Limits) -> Option<Self> {
        let fields = limits.type_fields.filter(|max| ty.fields > *max);
        let methods = limits.type_methods.filter(|max| ty.methods > *max);
        let over: Vec<_> = [
            fields.map(|max| format!("{} fields (>{max})", ty.fields)),
            methods.map(|max| format!("{} methods (>{max})", ty.methods)),
        ]
        .into_iter()
        .flatten()
        .collect();
        if over.is_empty() {
            return None;
        }
        Some(
            Self::new(
                path.to_path_buf(),
                WarningKind::LargeType,
                format!(
                    "{} {} at L{}-{}: {} — split responsibilities into smaller types",
                    ty.kind,
                    ty.name,
                    ty.start_line,
                    ty.end_line,
                    over.join(", ")
                ),
            )
            .at_line(ty.start_line)
            .about(ty.name.clone()),
        )
    }

    fn for_large_function(func: &TopFn, limits: &Limits) -> Option<Self> {
        let long = func.lines > limits.fn_lines;
        let heavy = limits.fn_tokens.filter(|max| func.tokens > *max);
        if !long && heavy.is_none() {
            return None;
        }
        let lines = if long {
            format!("{} lines (>{})", func.lines, limits.fn_lines)
        } else {
            format!("{} lines", func.lines)
        };
        let tokens = match heavy {
            Some(max) => format!("{} tokens (>{max})", func.tokens),
            None => format!("{} tokens", func.tokens),
        };
        let split = match func.largest_block {
            Some(span) => format!(" (largest block at L{}-{})", span.start, span.end),
            None => String::new(),
        };
        Some(Self::function(
            func,
            WarningKind::LargeFunction,
            format!("{lines}, {tokens} — consider extracting helpers{split}"),
        ))
    }

    fn for_deep_function(func: &TopFn, limits: &Limits) -> Option<Self> {
        let max = limits.fn_depth.filter(|max| func.depth > *max)?;
        let at = match func.deepest_block {
            Some(span) => format!(", deepest at L{}-{}", span.start, span.end),
            None => String::new(),
        };
        Some(Self::function(
            func,
            WarningKind::DeepFunction,
            format!(
                "nesting depth {} (>{max}{at}) — use early returns or extract the inner blocks",
                func.depth
            ),
        ))
    }

    fn for_many_params(func: &TopFn, limits: &Limits) -> Option<Self> {
        let max = limits.fn_params?;
        let params = func.params.filter(|p| *p > max)?;
        Some(Self::function(
            func,
            WarningKind::TooManyParams,
            format!("{params} parameters (>{max}) — group them in an input struct"),
        ))
    }

    fn for_many_bool_params(func: &TopFn, limits: &Limits) -> Option<Self> {
        let max = limits.fn_bool_params?;
        let bools = func.bool_params.filter(|p| *p > max)?;
        Some(Self::function(
            func,
            WarningKind::TooManyBoolParams,
            format!(
                "{bools} bool parameters (>{max}) — replace flags with an enum or options struct"
            ),
        ))
    }

    fn for_complex_function(func: &TopFn, limits: &Limits) -> Option<Self> {
        let cyclomatic = limits.fn_cyclomatic.filter(|max| func.cyclomatic > *max);
        let cognitive = limits.fn_cognitive.filter(|max| func.cognitive > *max);
        let over: Vec<_> = [
            cyclomatic.map(|max| format!("cyclomatic {} (>{max})", func.cyclomatic)),
            cognitive.map(|max| format!("cognitive {} (>{max})", func.cognitive)),
        ]
        .into_iter()
        .flatten()
        .collect();
        if over.is_empty() {
            return None;
        }
        Some(Self::function(
            func,
            WarningKind::ComplexFunction,
            format!(
                "complexity {} — split the branches into named helpers",
                over.join(", ")
            ),
        ))
    }

    fn function(func: &TopFn, kind: WarningKind, detail: String) -> Self {
        Self::new(
            func.path.clone(),
            kind,
            format!(
                "{} {} at L{}-{}: {detail}",
                func.kind, func.name, func.start_line, func.end_line
            ),
        )
        .at_line(func.start_line)
        .about(func.subject_key.clone())
    }
}
