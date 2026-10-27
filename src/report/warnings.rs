use std::cmp::Reverse;

use super::model::{FileLimits, TopFn, Warning};
use crate::count::FileStats;
use crate::units::Lines;

impl Warning {
    /// Files over the token or line limit, in report order.
    pub(super) fn for_files(files: &[FileStats], limits: &FileLimits) -> Vec<Self> {
        files
            .iter()
            .filter_map(|f| Self::for_file(f, limits))
            .collect()
    }

    /// Functions over the line limit, in ranked order.
    pub(super) fn for_functions(functions: &[TopFn], max_lines: Lines) -> Vec<Self> {
        functions
            .iter()
            .filter(|x| x.lines > max_lines)
            .map(|x| Self::for_function(x, max_lines))
            .collect()
    }

    fn for_file(f: &FileStats, limits: &FileLimits) -> Option<Self> {
        let FileLimits { tokens, lines } = limits;
        let mut over = Vec::new();
        if f.tokens > *tokens {
            over.push(format!("{} tokens (>{tokens})", f.tokens));
        }
        if f.lines > *lines {
            over.push(format!("{} lines (>{lines})", f.lines));
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
        Some(Self {
            path: f.path.clone(),
            kind: "large_file",
            message: format!("{} — consider splitting{hint}", over.join(", ")),
        })
    }

    fn for_function(func: &TopFn, max_lines: Lines) -> Self {
        Self {
            path: func.path.clone(),
            kind: "large_function",
            message: format!(
                "{} {} at L{}-{}: {} lines (>{}), {} tokens — consider extracting helpers",
                func.kind,
                func.name,
                func.start_line,
                func.end_line,
                func.lines,
                max_lines,
                func.tokens
            ),
        }
    }
}
