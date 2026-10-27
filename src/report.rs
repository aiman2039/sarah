use std::cmp::Reverse;
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::PathBuf;

use clap::ValueEnum;
use comfy_table::{Table, presets::UTF8_FULL_CONDENSED};
use serde::Serialize;

use crate::count::FileStats;

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum SortBy {
    Tokens,
    Lines,
    Name,
}

pub struct Thresholds {
    pub max_file_tokens: usize,
    pub max_file_lines: usize,
    pub max_fn_lines: usize,
}

#[derive(Debug, Default, Serialize)]
pub struct Totals {
    pub files: usize,
    pub lines: usize,
    pub code: usize,
    pub comments: usize,
    pub blanks: usize,
    pub tokens: usize,
    pub bytes: usize,
}

impl Totals {
    fn add(&mut self, f: &FileStats) {
        self.files += 1;
        self.lines += f.lines;
        self.code += f.code;
        self.comments += f.comments;
        self.blanks += f.blanks;
        self.tokens += f.tokens;
        self.bytes += f.bytes;
    }
}

#[derive(Debug, Serialize)]
pub struct TopFn {
    pub path: PathBuf,
    pub name: String,
    pub kind: String,
    pub start_line: usize,
    pub end_line: usize,
    pub lines: usize,
    pub tokens: usize,
}

#[derive(Debug, Serialize)]
pub struct Warning {
    pub path: PathBuf,
    pub kind: &'static str,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct Report {
    pub tokenizer: String,
    pub totals: Totals,
    pub by_language: BTreeMap<String, Totals>,
    pub files: Vec<FileStats>,
    /// All functions across the repo, largest (by lines) first.
    pub functions: Vec<TopFn>,
    pub warnings: Vec<Warning>,
}

impl Report {
    pub fn build(
        mut files: Vec<FileStats>,
        sort: SortBy,
        limits: &Thresholds,
        tokenizer: String,
    ) -> Self {
        match sort {
            SortBy::Tokens => files.sort_by_key(|f| Reverse(f.tokens)),
            SortBy::Lines => files.sort_by_key(|f| Reverse(f.lines)),
            SortBy::Name => files.sort_by(|a, b| a.path.cmp(&b.path)),
        }

        let mut totals = Totals::default();
        let mut by_language: BTreeMap<String, Totals> = BTreeMap::new();
        let mut functions = Vec::new();
        let mut warnings = Vec::new();

        for f in &files {
            totals.add(f);
            by_language.entry(f.language.clone()).or_default().add(f);
            for func in f.functions.as_deref().unwrap_or_default() {
                functions.push(TopFn {
                    path: f.path.clone(),
                    name: func.name.clone(),
                    kind: func.kind.clone(),
                    start_line: func.start_line,
                    end_line: func.end_line,
                    lines: func.lines,
                    tokens: func.tokens,
                });
            }
            warnings.extend(file_warning(f, limits));
        }

        functions.sort_by_key(|x| Reverse((x.lines, x.tokens)));
        warnings.extend(
            functions
                .iter()
                .filter(|x| x.lines > limits.max_fn_lines)
                .map(|x| function_warning(x, limits)),
        );

        Self {
            tokenizer,
            totals,
            by_language,
            files,
            functions,
            warnings,
        }
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("report serializes")
    }

    pub fn to_table(&self, top: usize) -> String {
        let mut out = String::new();
        let t = &self.totals;
        let _ = writeln!(
            out,
            "{} files, {} lines ({} code, {} comments, {} blank), {} tokens [{}]\n",
            t.files, t.lines, t.code, t.comments, t.blanks, t.tokens, self.tokenizer
        );

        let mut langs = new_table(&["Language", "Files", "Lines", "Code", "Tokens"]);
        for (name, l) in self.languages_by_tokens() {
            langs.add_row(vec![
                name.clone(),
                l.files.to_string(),
                l.lines.to_string(),
                l.code.to_string(),
                l.tokens.to_string(),
            ]);
        }
        let _ = writeln!(out, "{langs}\n");

        let mut files = new_table(&["File", "Lines", "Code", "Tokens", "Fns"]);
        for f in self.files.iter().take(top) {
            files.add_row(vec![
                f.path.display().to_string(),
                f.lines.to_string(),
                f.code.to_string(),
                f.tokens.to_string(),
                fn_count(f),
            ]);
        }
        let _ = writeln!(out, "Files (top {top}):\n{files}\n");

        if !self.functions.is_empty() {
            let mut fns = new_table(&["Function", "Location", "Lines", "Tokens"]);
            for f in self.functions.iter().take(top) {
                fns.add_row(vec![
                    f.name.clone(),
                    format!("{}:{}", f.path.display(), f.start_line),
                    f.lines.to_string(),
                    f.tokens.to_string(),
                ]);
            }
            let _ = writeln!(out, "Functions (top {top} by lines):\n{fns}\n");
        }

        self.write_warnings(&mut out, "Warnings", "- ");
        out
    }

    pub fn to_markdown(&self, top: usize) -> String {
        let mut out = String::from("# Repo size report\n\n");
        let t = &self.totals;
        let _ = writeln!(
            out,
            "**{} files**, {} lines ({} code, {} comments, {} blank), **{} tokens** (tokenizer: {})\n",
            t.files, t.lines, t.code, t.comments, t.blanks, t.tokens, self.tokenizer
        );

        out.push_str("## Languages\n\n| Language | Files | Lines | Code | Tokens |\n|---|---:|---:|---:|---:|\n");
        for (name, l) in self.languages_by_tokens() {
            let _ = writeln!(
                out,
                "| {name} | {} | {} | {} | {} |",
                l.files, l.lines, l.code, l.tokens
            );
        }

        let _ = writeln!(
            out,
            "\n## Files (top {top})\n\n| File | Lines | Code | Tokens | Fns |\n|---|---:|---:|---:|---:|"
        );
        for f in self.files.iter().take(top) {
            let _ = writeln!(
                out,
                "| `{}` | {} | {} | {} | {} |",
                f.path.display(),
                f.lines,
                f.code,
                f.tokens,
                fn_count(f)
            );
        }

        if !self.functions.is_empty() {
            let _ = writeln!(
                out,
                "\n## Functions (top {top} by lines)\n\n| Function | Location | Lines | Tokens |\n|---|---|---:|---:|"
            );
            for f in self.functions.iter().take(top) {
                let _ = writeln!(
                    out,
                    "| `{}` | `{}:{}` | {} | {} |",
                    f.name,
                    f.path.display(),
                    f.start_line,
                    f.lines,
                    f.tokens
                );
            }
        }

        out.push('\n');
        self.write_warnings(&mut out, "## Refactor candidates", "- ");
        out
    }

    fn languages_by_tokens(&self) -> Vec<(&String, &Totals)> {
        let mut langs: Vec<_> = self.by_language.iter().collect();
        langs.sort_by_key(|(_, l)| Reverse(l.tokens));
        langs
    }

    fn write_warnings(&self, out: &mut String, heading: &str, bullet: &str) {
        if self.warnings.is_empty() {
            let _ = writeln!(out, "{heading}: none");
            return;
        }
        let _ = writeln!(out, "{heading} ({}):\n", self.warnings.len());
        for w in &self.warnings {
            let _ = writeln!(out, "{bullet}{}: {}", w.path.display(), w.message);
        }
    }
}

fn file_warning(f: &FileStats, limits: &Thresholds) -> Option<Warning> {
    let mut over = Vec::new();
    if f.tokens > limits.max_file_tokens {
        over.push(format!("{} tokens (>{})", f.tokens, limits.max_file_tokens));
    }
    if f.lines > limits.max_file_lines {
        over.push(format!("{} lines (>{})", f.lines, limits.max_file_lines));
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
    Some(Warning {
        path: f.path.clone(),
        kind: "large_file",
        message: format!("{} — consider splitting{hint}", over.join(", ")),
    })
}

fn function_warning(func: &TopFn, limits: &Thresholds) -> Warning {
    Warning {
        path: func.path.clone(),
        kind: "large_function",
        message: format!(
            "{} {} at L{}-{}: {} lines (>{}), {} tokens — consider extracting helpers",
            func.kind,
            func.name,
            func.start_line,
            func.end_line,
            func.lines,
            limits.max_fn_lines,
            func.tokens
        ),
    }
}

fn new_table(header: &[&str]) -> Table {
    let mut table = Table::new();
    table.load_style(UTF8_FULL_CONDENSED).set_header(header);
    table
}

fn fn_count(f: &FileStats) -> String {
    f.functions
        .as_ref()
        .map_or("-".to_string(), |v| v.len().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::functions::FnStats;

    fn file(path: &str, lines: usize, tokens: usize, fn_lines: usize) -> FileStats {
        FileStats {
            path: path.into(),
            language: "Rust".into(),
            bytes: 0,
            lines,
            code: lines,
            comments: 0,
            blanks: 0,
            tokens,
            functions: Some(vec![FnStats {
                name: "big".into(),
                kind: "function".into(),
                start_line: 1,
                end_line: fn_lines,
                lines: fn_lines,
                tokens: 10,
            }]),
        }
    }

    #[test]
    fn thresholds_fire() {
        let limits = Thresholds {
            max_file_tokens: 100,
            max_file_lines: 50,
            max_fn_lines: 20,
        };
        let r = Report::build(
            vec![file("ok.rs", 10, 10, 5), file("big.rs", 60, 200, 30)],
            SortBy::Tokens,
            &limits,
            "o200k".into(),
        );
        assert_eq!(r.totals.files, 2);
        assert_eq!(r.totals.tokens, 210);
        assert_eq!(r.files[0].path, PathBuf::from("big.rs"));
        let kinds: Vec<_> = r.warnings.iter().map(|w| (w.path.clone(), w.kind)).collect();
        assert_eq!(
            kinds,
            vec![
                ("big.rs".into(), "large_file"),
                ("big.rs".into(), "large_function")
            ]
        );
    }
}
