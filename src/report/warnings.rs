use std::cmp::Reverse;

use super::model::{TopFn, Warning, WarningKind};
use crate::config::{LimitRules, Limits};
use crate::count::FileStats;

impl Warning {
    /// Files over their token or line limit, in report order.
    pub(super) fn for_files(files: &[FileStats], rules: &LimitRules) -> Vec<Self> {
        files
            .iter()
            .filter_map(|f| Self::for_file(f, &rules.for_path(&f.path)))
            .collect()
    }

    /// Functions over their size or nesting limits, in ranked order.
    pub(super) fn for_functions(functions: &[TopFn], rules: &LimitRules) -> Vec<Self> {
        functions
            .iter()
            .flat_map(|func| {
                let limits = rules.for_path(&func.path);
                [
                    Self::for_large_function(func, &limits),
                    Self::for_deep_function(func, &limits),
                ]
            })
            .flatten()
            .collect()
    }

    /// Malformed or misplaced `sarah-allow` comments, file by file.
    pub(super) fn for_allow_problems(files: &[FileStats]) -> Vec<Self> {
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
        Some(Self::function(
            func,
            WarningKind::LargeFunction,
            format!("{lines}, {tokens} — consider extracting helpers"),
        ))
    }

    fn for_deep_function(func: &TopFn, limits: &Limits) -> Option<Self> {
        let max = limits.fn_depth.filter(|max| func.depth > *max)?;
        Some(Self::function(
            func,
            WarningKind::DeepFunction,
            format!(
                "nesting depth {} (>{max}) — use early returns or extract the inner blocks",
                func.depth
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
        .about(func.name.clone())
    }
}
