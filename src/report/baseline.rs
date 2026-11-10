use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Deserialize;

use super::model::Report;
use super::warning::{Warning, WarningKind};
use crate::structure::DirStats;
use crate::units::{Cognitive, Cyclomatic, Depth, LineNo, Lines, Params, Tokens};

/// A JSON report from an earlier run. Its warnings are hidden unless the measured
/// sizes grew since.
pub struct Baseline {
    known: HashSet<Key>,
    sizes: Sizes,
}

type Key = (PathBuf, WarningKind, Option<String>);

/// Lines, tokens, depth, params, bool_params, cyclomatic and cognitive complexity.
type FnSizes = [usize; 7];

#[derive(Deserialize)]
struct SavedReport {
    files: Vec<SavedFile>,
    functions: Vec<SavedFn>,
    #[serde(default)]
    directories: Vec<DirStats>,
    warnings: Vec<SavedWarning>,
}

#[derive(Deserialize)]
struct SavedFile {
    path: PathBuf,
    tokens: Tokens,
    lines: Lines,
}

#[derive(Deserialize)]
struct SavedFn {
    path: PathBuf,
    name: String,
    #[serde(default)]
    start_line: Option<LineNo>,
    lines: Lines,
    tokens: Tokens,
    #[serde(default)]
    depth: Depth,
    #[serde(default)]
    params: Option<Params>,
    #[serde(default)]
    bool_params: Option<Params>,
    #[serde(default)]
    cyclomatic: Cyclomatic,
    #[serde(default)]
    cognitive: Cognitive,
}

#[derive(Deserialize)]
struct SavedWarning {
    path: PathBuf,
    kind: WarningKind,
    #[serde(default)]
    subject: Option<String>,
}

/// What each warning measures. A warning got worse when any of its sizes grew.
#[derive(Default)]
struct Sizes {
    files: HashMap<PathBuf, [usize; 3]>,
    functions: HashMap<(PathBuf, String), FnSizes>,
    dirs: HashMap<PathBuf, [usize; 5]>,
    dir_max_depth: HashMap<PathBuf, usize>,
    dir_inv_entries: HashMap<PathBuf, usize>,
}

impl Baseline {
    pub fn load(path: &Path) -> Result<Self> {
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("cannot read baseline {}", path.display()))?;
        let saved: SavedReport = serde_json::from_str(&text)
            .with_context(|| format!("{} is not a sarah JSON report", path.display()))?;
        Ok(Self::from_saved(saved))
    }

    fn from_saved(saved: SavedReport) -> Self {
        let SavedReport {
            files,
            functions,
            directories,
            warnings,
        } = saved;
        let mut sizes = Sizes::default();
        let mut fn_counts: HashMap<PathBuf, usize> = HashMap::new();
        for f in &functions {
            let count = fn_counts.entry(f.path.clone()).or_insert(0);
            *count = count.saturating_add(1);
        }
        for f in files {
            let fns = fn_counts.get(&f.path).copied().unwrap_or(0);
            sizes.files.insert(f.path, [f.tokens.0, f.lines.0, fns]);
        }
        sizes.add_saved_fns(functions);
        sizes.add_dirs(&directories);
        let known = warnings
            .into_iter()
            .map(|w| (w.path, w.kind, w.subject))
            .collect();
        Self { known, sizes }
    }

    fn covers(&self, warning: &Warning, current: &Sizes) -> bool {
        let key = (warning.path.clone(), warning.kind, warning.subject.clone());
        if !self.known.contains(&key) {
            return false;
        }
        match (current.of(warning), self.sizes.of(warning)) {
            (Some(now), Some(then)) => now.iter().zip(then).all(|(n, t)| *n <= t),
            _ => true,
        }
    }
}

impl Sizes {
    fn of_report(report: &Report) -> Self {
        let mut sizes = Self::default();
        for f in &report.files {
            let fns = f.functions.as_ref().map_or(0, Vec::len);
            sizes
                .files
                .insert(f.path.clone(), [f.tokens.0, f.lines.0, fns]);
        }
        for f in &report.functions {
            let fn_sizes = [
                f.lines.0,
                f.tokens.0,
                f.depth.0,
                f.params.map_or(0, |p| p.0),
                f.bool_params.map_or(0, |p| p.0),
                f.cyclomatic.0,
                f.cognitive.0,
            ];
            sizes
                .functions
                .insert((f.path.clone(), f.subject_key.clone()), fn_sizes);
            sizes.add_max_fn((f.path.clone(), f.name.clone()), fn_sizes);
        }
        sizes.add_dirs(&report.directories);
        sizes
    }

    fn add_saved_fns(&mut self, mut functions: Vec<SavedFn>) {
        functions.sort_by_key(|f| (f.path.clone(), f.start_line.unwrap_or(LineNo(0))));
        let mut totals: HashMap<(PathBuf, String), usize> = HashMap::new();
        for f in &functions {
            let count = totals.entry((f.path.clone(), f.name.clone())).or_insert(0);
            *count = count.saturating_add(1);
        }
        let mut seen: HashMap<(PathBuf, String), usize> = HashMap::new();
        for f in functions {
            let fn_sizes = [
                f.lines.0,
                f.tokens.0,
                f.depth.0,
                f.params.map_or(0, |p| p.0),
                f.bool_params.map_or(0, |p| p.0),
                f.cyclomatic.0,
                f.cognitive.0,
            ];
            let pair = (f.path.clone(), f.name.clone());
            let ord = seen.entry(pair.clone()).or_insert(0);
            *ord = ord.saturating_add(1);
            if totals.get(&pair).copied().unwrap_or(0) > 1 {
                let disambig = format!("{}#{ord}", f.name);
                self.functions.insert((f.path, disambig), fn_sizes);
            }
            self.add_max_fn(pair, fn_sizes);
        }
    }

    fn add_max_fn(&mut self, key: (PathBuf, String), sizes: FnSizes) {
        let entry = self.functions.entry(key).or_default();
        for (e, s) in entry.iter_mut().zip(sizes) {
            *e = (*e).max(s);
        }
    }

    fn add_dirs(&mut self, dirs: &[DirStats]) {
        for d in dirs {
            self.dirs.insert(
                d.path.clone(),
                [d.files, d.total_files, d.dirs, d.lines.0, d.tokens.0],
            );
            let entries = d.files.saturating_add(d.dirs);
            self.dir_inv_entries
                .insert(d.path.clone(), usize::MAX.saturating_sub(entries));
            let max_depth = dirs
                .iter()
                .filter(|other| d.path == Path::new(".") || other.path.starts_with(&d.path))
                .map(|other| other.depth.0)
                .max()
                .unwrap_or(d.depth.0);
            self.dir_max_depth.insert(d.path.clone(), max_depth);
        }
    }

    fn of(&self, w: &Warning) -> Option<Vec<usize>> {
        match w.kind {
            WarningKind::LargeFile | WarningKind::TooManyFunctions => {
                self.files.get(&w.path).map(|s| s.to_vec())
            }
            WarningKind::LargeFunction
            | WarningKind::DeepFunction
            | WarningKind::TooManyParams
            | WarningKind::TooManyBoolParams
            | WarningKind::ComplexFunction => {
                let name = w.subject.clone()?;
                self.functions
                    .get(&(w.path.clone(), name))
                    .map(|s| s.to_vec())
            }
            WarningKind::FolderTooManyFiles
            | WarningKind::FolderTooManyTotalFiles
            | WarningKind::FolderTooManyDirs
            | WarningKind::FolderTooManyLines
            | WarningKind::FolderTooManyTokens => self.dirs.get(&w.path).map(|s| s.to_vec()),
            WarningKind::FolderTooDeep => self.dir_max_depth.get(&w.path).map(|d| vec![*d]),
            WarningKind::FolderTooFewEntries => self.dir_inv_entries.get(&w.path).map(|e| vec![*e]),
            WarningKind::TooManyImports
            | WarningKind::LargeType
            | WarningKind::ForbiddenImport
            | WarningKind::CircularImport
            | WarningKind::DuplicateCode
            | WarningKind::TodoComment
            | WarningKind::FolderMissingRequired
            | WarningKind::FolderForbiddenName
            | WarningKind::FolderBadName
            | WarningKind::FolderSiblingModule
            | WarningKind::FolderUnexpectedDir
            | WarningKind::BadAllow
            | WarningKind::UnusedAllow => None,
        }
    }
}

impl Report {
    /// Drop warnings the baseline already had, unless they got worse.
    pub fn apply_baseline(&mut self, baseline: &Baseline) {
        let current = Sizes::of_report(self);
        let before = self.warnings.len();
        self.warnings.retain(|w| !baseline.covers(w, &current));
        self.baselined = Some(before.saturating_sub(self.warnings.len()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn saved(json: &str) -> Baseline {
        Baseline::from_saved(serde_json::from_str(json).unwrap())
    }

    fn warning(kind: WarningKind, subject: Option<&str>) -> Warning {
        let w = Warning::new("a.rs".into(), kind, String::new());
        match subject {
            Some(s) => w.about(s.into()),
            None => w,
        }
    }

    #[test]
    fn known_warnings_hide_until_they_grow_including_duplicate_names() {
        let baseline = saved(
            r#"{"files":[{"path":"a.rs","tokens":100,"lines":10}],
                "functions":[
                  {"path":"a.rs","name":"new","start_line":1,"lines":80,"tokens":50},
                  {"path":"a.rs","name":"new","start_line":90,"lines":20,"tokens":15}
                ],
                "directories":[{"path":"src/a","depth":3,"files":1,"dirs":1,"total_files":2,"lines":10,"tokens":20}],
                "warnings":[
                  {"path":"a.rs","kind":"large_function","subject":"new#1","message":""},
                  {"path":"a.rs","kind":"large_function","subject":"new#2","message":""},
                  {"path":"src/a","kind":"folder_too_deep","message":""}
                ]}"#,
        );
        let mut current = Sizes::default();
        current.files.insert("a.rs".into(), [100, 10, 2]);
        current
            .functions
            .insert(("a.rs".into(), "new#1".into()), [80, 50, 0, 0, 0, 0, 0]);
        current
            .functions
            .insert(("a.rs".into(), "new#2".into()), [25, 15, 0, 0, 0, 0, 0]);
        current.dir_max_depth.insert("src/a".into(), 4);

        assert!(baseline.covers(
            &warning(WarningKind::LargeFunction, Some("new#1")),
            &current
        ));
        assert!(!baseline.covers(
            &warning(WarningKind::LargeFunction, Some("new#2")),
            &current
        ));
        let deep = Warning::new("src/a".into(), WarningKind::FolderTooDeep, String::new());
        assert!(!baseline.covers(&deep, &current));
    }
}
