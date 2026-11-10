use std::cmp::Reverse;
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

use clap::ValueEnum;
use serde::Serialize;

use super::diff::DiffSummary;
use super::warning::{Warning, WarningKind};
use crate::config::LimitRules;
use crate::count::FileStats;
use crate::duplicate::DuplicateHelpers;
use crate::functions::FnStats;
use crate::graph::GraphHelpers;
use crate::structure::{DirStats, DirTree, FolderRules};
use crate::units::{Bytes, Cognitive, Cyclomatic, Depth, LineNo, LineSpan, Lines, Params, Tokens};
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
    pub no_cycles: bool,
    pub max_duplicate_lines: Option<Lines>,
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
    #[serde(skip_serializing_if = "FnStats::skip_bool_params")]
    pub bool_params: Option<Params>,
    pub cyclomatic: Cyclomatic,
    pub cognitive: Cognitive,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub allow: Vec<WarningKind>,
    #[serde(skip)]
    pub deepest_block: Option<LineSpan>,
    #[serde(skip)]
    pub largest_block: Option<LineSpan>,
    /// `name` when unique in `path`, else `name#1`, `name#2`, ... in line order.
    #[serde(skip)]
    pub subject_key: String,
}

impl TopFn {
    /// Every function across `files`, largest (lines, then tokens) first.
    fn ranked(files: &[FileStats]) -> Vec<Self> {
        let mut functions: Vec<Self> = files.iter().flat_map(Self::of_file).collect();
        functions.sort_by_key(|x| Reverse((x.lines, x.tokens)));
        functions
    }

    fn of_file(file: &FileStats) -> Vec<Self> {
        let slice = file.functions.as_deref().unwrap_or_default();
        let mut sorted: Vec<&FnStats> = slice.iter().collect();
        sorted.sort_by_key(|f| f.start_line);
        let mut totals: HashMap<&str, usize> = HashMap::new();
        for func in &sorted {
            let entry = totals.entry(func.name.as_str()).or_insert(0);
            *entry = entry.saturating_add(1);
        }
        let mut seen: HashMap<&str, usize> = HashMap::new();
        sorted
            .into_iter()
            .map(|func| {
                let ord = seen.entry(func.name.as_str()).or_insert(0);
                *ord = ord.saturating_add(1);
                let subject_key = if totals.get(func.name.as_str()).copied().unwrap_or(0) > 1 {
                    format!("{}#{ord}", func.name)
                } else {
                    func.name.clone()
                };
                Self::from_stats(&file.path, func, subject_key)
            })
            .collect()
    }

    fn from_stats(path: &Path, func: &FnStats, subject_key: String) -> Self {
        Self {
            path: path.to_path_buf(),
            name: func.name.clone(),
            kind: func.kind.clone(),
            start_line: func.start_line,
            end_line: func.end_line,
            lines: func.lines,
            tokens: func.tokens,
            depth: func.depth,
            params: func.params,
            bool_params: func.bool_params,
            cyclomatic: func.cyclomatic,
            cognitive: func.cognitive,
            allow: func.allow.clone(),
            deepest_block: func.deepest_block,
            largest_block: func.largest_block,
            subject_key,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct Report {
    pub tokenizer: String,
    pub totals: Totals,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diff: Option<DiffSummary>,
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
            no_cycles,
            max_duplicate_lines,
        } = input;
        sort.apply(&mut files);
        let totals = Totals::of(&files);
        let by_language = Totals::by_language(&files);
        let functions = TopFn::ranked(&files);
        let tree = DirTree::build(&files);
        let warnings = [
            Warning::for_files(&files, &limits),
            Warning::for_types(&files, &limits),
            Warning::for_imports(&files, &limits, &folders),
            Warning::for_comments(&files, &limits),
            Warning::for_functions(&functions, &limits),
            folders.check(&tree),
            GraphHelpers::check_cycles(&files, no_cycles),
            DuplicateHelpers::check(&files, max_duplicate_lines),
            Warning::for_allow_problems(&files, &limits),
        ]
        .into_iter()
        .flatten()
        .collect();
        Self {
            tokenizer,
            totals,
            diff: None,
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
