use std::cmp::Reverse;
use std::collections::BTreeMap;
use std::path::PathBuf;

use clap::ValueEnum;
use serde::Serialize;

use crate::count::FileStats;
use crate::units::{Bytes, LineNo, Lines, Tokens};
use crate::walk::Skipped;

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum SortBy {
    Tokens,
    Lines,
    Name,
}

pub struct Thresholds {
    pub file: FileLimits,
    pub fn_lines: Lines,
}

pub struct FileLimits {
    pub tokens: Tokens,
    pub lines: Lines,
}

pub struct ReportInput {
    pub files: Vec<FileStats>,
    pub skipped: Skipped,
    pub sort: SortBy,
    pub limits: Thresholds,
    pub tokenizer: String,
}

#[derive(Debug, Default, Serialize)]
pub struct Totals {
    pub files: usize,
    pub lines: Lines,
    pub code: Lines,
    pub comments: Lines,
    pub blanks: Lines,
    pub tokens: Tokens,
    pub bytes: Bytes,
}

impl SortBy {
    fn apply(self, files: &mut [FileStats]) {
        match self {
            Self::Tokens => files.sort_by_key(|f| Reverse(f.tokens)),
            Self::Lines => files.sort_by_key(|f| Reverse(f.lines)),
            Self::Name => files.sort_by(|a, b| a.path.cmp(&b.path)),
        }
    }
}

impl Totals {
    fn of(files: &[FileStats]) -> Self {
        let mut totals = Self::default();
        for f in files {
            totals.add(f);
        }
        totals
    }

    fn by_language(files: &[FileStats]) -> BTreeMap<String, Self> {
        let mut by_language: BTreeMap<String, Self> = BTreeMap::new();
        for f in files {
            by_language.entry(f.language.clone()).or_default().add(f);
        }
        by_language
    }

    fn add(&mut self, f: &FileStats) {
        self.files = self.files.saturating_add(1);
        self.lines = self.lines.saturating_add(f.lines);
        self.code = self.code.saturating_add(f.code);
        self.comments = self.comments.saturating_add(f.comments);
        self.blanks = self.blanks.saturating_add(f.blanks);
        self.tokens = self.tokens.saturating_add(f.tokens);
        self.bytes = self.bytes.saturating_add(f.bytes);
    }
}

#[derive(Debug, Serialize)]
pub struct TopFn {
    pub path: PathBuf,
    pub name: String,
    pub kind: String,
    pub start_line: LineNo,
    pub end_line: LineNo,
    pub lines: Lines,
    pub tokens: Tokens,
}

impl TopFn {
    /// Every function across `files`, largest (lines, then tokens) first.
    fn ranked(files: &[FileStats]) -> Vec<Self> {
        let mut functions: Vec<Self> = files
            .iter()
            .flat_map(|f| {
                f.functions
                    .as_deref()
                    .unwrap_or_default()
                    .iter()
                    .map(|func| Self {
                        path: f.path.clone(),
                        name: func.name.clone(),
                        kind: func.kind.clone(),
                        start_line: func.start_line,
                        end_line: func.end_line,
                        lines: func.lines,
                        tokens: func.tokens,
                    })
            })
            .collect();
        functions.sort_by_key(|x| Reverse((x.lines, x.tokens)));
        functions
    }
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
    pub skipped: Skipped,
}

impl Report {
    pub fn build(input: ReportInput) -> Self {
        let ReportInput {
            mut files,
            skipped,
            sort,
            limits: Thresholds { file, fn_lines },
            tokenizer,
        } = input;
        sort.apply(&mut files);
        let totals = Totals::of(&files);
        let by_language = Totals::by_language(&files);
        let functions = TopFn::ranked(&files);
        let warnings = [
            Warning::for_files(&files, &file),
            Warning::for_functions(&functions, fn_lines),
        ]
        .into_iter()
        .flatten()
        .collect();
        Self {
            tokenizer,
            totals,
            by_language,
            files,
            functions,
            warnings,
            skipped,
        }
    }

    pub fn to_json(&self) -> serde_json::Result<String> {
        serde_json::to_string_pretty(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::functions::FnStats;

    struct Fixture {
        path: &'static str,
        lines: Lines,
        tokens: Tokens,
        fn_lines: Lines,
    }

    fn file(fixture: Fixture) -> FileStats {
        let Fixture {
            path,
            lines,
            tokens,
            fn_lines,
        } = fixture;
        FileStats {
            path: path.into(),
            language: "Rust".into(),
            bytes: Bytes(0),
            lines,
            code: lines,
            comments: Lines(0),
            blanks: Lines(0),
            tokens,
            functions: Some(vec![FnStats {
                name: "big".into(),
                kind: "function".into(),
                start_line: LineNo(1),
                end_line: LineNo(fn_lines.0),
                lines: fn_lines,
                tokens: Tokens(10),
            }]),
        }
    }

    #[test]
    fn thresholds_fire() {
        let r = Report::build(ReportInput {
            files: vec![
                file(Fixture {
                    path: "ok.rs",
                    lines: Lines(10),
                    tokens: Tokens(10),
                    fn_lines: Lines(5),
                }),
                file(Fixture {
                    path: "big.rs",
                    lines: Lines(60),
                    tokens: Tokens(200),
                    fn_lines: Lines(30),
                }),
            ],
            skipped: Skipped::default(),
            sort: SortBy::Tokens,
            limits: Thresholds {
                file: FileLimits {
                    tokens: Tokens(100),
                    lines: Lines(50),
                },
                fn_lines: Lines(20),
            },
            tokenizer: "o200k".into(),
        });
        assert_eq!(r.totals.files, 2);
        assert_eq!(r.totals.tokens, Tokens(210));
        assert_eq!(r.files[0].path, PathBuf::from("big.rs"));
        let kinds: Vec<_> = r
            .warnings
            .iter()
            .map(|w| (w.path.clone(), w.kind))
            .collect();
        assert_eq!(
            kinds,
            vec![
                ("big.rs".into(), "large_file"),
                ("big.rs".into(), "large_function")
            ]
        );
    }
}
