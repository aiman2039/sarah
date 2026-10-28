use std::cmp::Reverse;
use std::collections::BTreeMap;
use std::path::PathBuf;

use clap::ValueEnum;
use derive_more::Display;
use serde::{Deserialize, Serialize};

use crate::config::LimitRules;
use crate::count::FileStats;
use crate::structure::{DirStats, DirTree, FolderRules};
use crate::units::{Bytes, Cognitive, Cyclomatic, Depth, LineNo, Lines, Params, Tokens};
use crate::walk::Skipped;

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum SortBy {
    Tokens,
    Lines,
    Name,
}

pub struct ReportInput {
    pub files: Vec<FileStats>,
    pub skipped: Skipped,
    pub sort: SortBy,
    pub limits: LimitRules,
    pub folders: FolderRules,
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
    pub depth: Depth,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Params>,
    pub cyclomatic: Cyclomatic,
    pub cognitive: Cognitive,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub allow: Vec<WarningKind>,
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
                        depth: func.depth,
                        params: func.params,
                        cyclomatic: func.cyclomatic,
                        cognitive: func.cognitive,
                        allow: func.allow.clone(),
                    })
            })
            .collect();
        functions.sort_by_key(|x| Reverse((x.lines, x.tokens)));
        functions
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Display, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[display(rename_all = "snake_case")]
pub enum WarningKind {
    LargeFile,
    LargeFunction,
    DeepFunction,
    /// More parameters than `max_fn_params`.
    TooManyParams,
    /// Over `max_fn_cyclomatic` or `max_fn_cognitive`.
    ComplexFunction,
    FolderTooManyFiles,
    FolderTooManyDirs,
    FolderTooDeep,
    FolderTooManyTokens,
    FolderTooFewEntries,
    FolderMissingRequired,
    FolderForbiddenName,
    FolderBadName,
    FolderSiblingModule,
    FolderUnexpectedDir,
    /// A malformed or misplaced `sarah-allow` comment.
    BadAllow,
}

impl WarningKind {
    /// Folder-structure warnings (as opposed to file and function warnings).
    pub fn is_folder(self) -> bool {
        self.to_string().starts_with("folder_")
    }
}

#[derive(Debug, Serialize)]
pub struct Warning {
    pub path: PathBuf,
    pub kind: WarningKind,
    pub message: String,
    /// First line of the offending function, when there is one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<LineNo>,
    /// What the warning is about inside `path` (a function name, a required glob);
    /// with `path` and `kind` it identifies the warning across runs (`--baseline`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,
}

impl Warning {
    pub fn new(path: PathBuf, kind: WarningKind, message: String) -> Self {
        Self {
            path,
            kind,
            message,
            line: None,
            subject: None,
        }
    }

    pub fn at_line(self, line: LineNo) -> Self {
        Self {
            line: Some(line),
            ..self
        }
    }

    pub fn about(self, subject: String) -> Self {
        Self {
            subject: Some(subject),
            ..self
        }
    }
}

#[derive(Debug, Serialize)]
pub struct Report {
    pub tokenizer: String,
    pub totals: Totals,
    pub by_language: BTreeMap<String, Totals>,
    pub files: Vec<FileStats>,
    /// All functions across the repo, largest (by lines) first.
    pub functions: Vec<TopFn>,
    /// Every folder holding an analyzed file, in path order.
    pub directories: Vec<DirStats>,
    pub warnings: Vec<Warning>,
    pub skipped: Skipped,
    /// Warnings hidden because `--baseline` already had them.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub baselined: Option<usize>,
    /// Warnings hidden by `sarah-allow` comments.
    #[serde(skip_serializing_if = "Report::is_zero")]
    pub allowed: usize,
}

impl Report {
    pub fn build(input: ReportInput) -> Self {
        let ReportInput {
            mut files,
            skipped,
            sort,
            limits,
            folders,
            tokenizer,
        } = input;
        sort.apply(&mut files);
        let totals = Totals::of(&files);
        let by_language = Totals::by_language(&files);
        let functions = TopFn::ranked(&files);
        let tree = DirTree::build(&files);
        let warnings = [
            Warning::for_files(&files, &limits),
            Warning::for_functions(&functions, &limits),
            folders.check(&tree),
            Warning::for_allow_problems(&files),
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
            directories: tree.stats(),
            warnings,
            skipped,
            baselined: None,
            allowed: 0,
        }
    }

    fn is_zero(n: &usize) -> bool {
        *n == 0
    }

    pub fn to_json(&self) -> serde_json::Result<String> {
        serde_json::to_string_pretty(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::allow::Allows;
    use crate::config::LimitPatch;
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
                depth: Depth(0),
                params: None,
                cyclomatic: Cyclomatic(1),
                cognitive: Cognitive(0),
                allow: Vec::new(),
            }]),
            allows: Allows::default(),
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
            limits: LimitRules::new(
                &[],
                LimitPatch {
                    max_file_tokens: Some(Tokens(100)),
                    max_file_lines: Some(Lines(50)),
                    max_fn_lines: Some(Lines(20)),
                    ..LimitPatch::default()
                },
            )
            .unwrap(),
            folders: FolderRules::new(None, Vec::new()).unwrap(),
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
                ("big.rs".into(), WarningKind::LargeFile),
                ("big.rs".into(), WarningKind::LargeFunction)
            ]
        );
    }
}
