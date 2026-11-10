use std::cmp::Reverse;
use std::collections::BTreeSet;
use std::path::Path;

use crate::config::{LimitRules, Limits, NamePatterns};
use crate::count::FileStats;
use crate::functions::{ImportHelpers, TypeStats};
use crate::report::warning::{Warning, WarningKind};
use crate::structure::FolderRules;
use crate::units::LineNo;

impl Warning {
    /// Files over their token, line, function-count or import-count limits.
    pub fn for_files(files: &[FileStats], rules: &LimitRules) -> Vec<Self> {
        files
            .iter()
            .flat_map(|f| {
                let limits = rules.for_path(&f.path);
                [
                    Self::for_file(f, &limits),
                    Self::for_file_fns(f, &limits),
                    Self::for_file_imports(f, &limits),
                ]
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

    /// Imports matching `forbid_imports` (raw or normalized relative path).
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

    /// `TODO`/`FIXME`/`HACK`/`XXX` caps (`max_todos`) and `forbid_comments` patterns.
    pub fn for_comments(files: &[FileStats], rules: &LimitRules) -> Vec<Self> {
        files
            .iter()
            .flat_map(|f| Self::file_comments(f, rules))
            .collect()
    }

    /// Malformed, misplaced or reason-less `sarah-allow` comments.
    pub fn for_allow_problems(files: &[FileStats], rules: &LimitRules) -> Vec<Self> {
        let mut out = Vec::new();
        for f in files {
            for p in &f.allows.problems {
                out.push(
                    Self::new(
                        f.path.clone(),
                        WarningKind::BadAllow,
                        format!("sarah-allow at L{}: {}", p.line, p.message),
                    )
                    .at_line(p.line),
                );
            }
            if rules.for_path(&f.path).require_allow_reason {
                out.extend(Self::missing_reason_warnings(f));
            }
        }
        out
    }

    fn missing_reason_warnings(f: &FileStats) -> Vec<Self> {
        let mut lines = BTreeSet::new();
        for tracked in &f.allows.tracked {
            if !tracked.has_reason {
                lines.insert(tracked.line);
            }
        }
        lines
            .into_iter()
            .map(|line: LineNo| {
                Self::new(
                    f.path.clone(),
                    WarningKind::BadAllow,
                    format!("sarah-allow at L{line}: missing reason after `:`"),
                )
                .at_line(line)
            })
            .collect()
    }

    fn file_comments(f: &FileStats, rules: &LimitRules) -> Vec<Self> {
        let mut out = Vec::new();
        if let Some(max) = rules.for_path(&f.path).todos
            && f.comment_info.todos.len() > max
        {
            let count = f.comment_info.todos.len();
            let first = f.comment_info.todos.first().map(|(l, _)| *l);
            let mut w = Self::new(
                f.path.clone(),
                WarningKind::TodoComment,
                format!("{count} TODO/FIXME comments (>{max}) — resolve or file an issue"),
            );
            if let Some(line) = first {
                w = w.at_line(line);
            }
            out.push(w);
        }
        if let Some(forbid) = rules.forbid_comments_for(&f.path) {
            out.extend(Self::forbidden_comments(f, forbid));
        }
        out
    }

    fn forbidden_comments(f: &FileStats, forbid: &NamePatterns) -> Vec<Self> {
        f.comment_info
            .lines
            .iter()
            .filter_map(|(line, text)| {
                let pat = forbid.find(text)?;
                Some(
                    Self::new(
                        f.path.clone(),
                        WarningKind::TodoComment,
                        format!("comment at L{line} matches forbidden {pat:?}"),
                    )
                    .at_line(*line)
                    .about(pat.to_string()),
                )
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
                let resolved = ImportHelpers::resolve_relative(&f.path, &imp.path);
                let pat = Self::match_import(by_file, &imp.path, resolved.as_deref())
                    .or_else(|| Self::match_import(by_folder, &imp.path, resolved.as_deref()))?;
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

    fn match_import<'a>(
        patterns: Option<&'a NamePatterns>,
        raw: &str,
        resolved: Option<&str>,
    ) -> Option<&'a str> {
        let p = patterns?;
        p.find(raw).or_else(|| resolved.and_then(|r| p.find(r)))
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

    fn for_file_imports(f: &FileStats, limits: &Limits) -> Option<Self> {
        let max = limits.file_imports?;
        let count = f.imports.len();
        (count > max.0).then(|| {
            Self::new(
                f.path.clone(),
                WarningKind::TooManyImports,
                format!("{count} imports (>{max}) — reduce coupling or split the file"),
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
}
