use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Deserialize;

use super::model::{Report, Warning, WarningKind};
use crate::structure::DirStats;
use crate::units::{Depth, Lines, Tokens};

/// A JSON report from an earlier run. Its warnings are hidden unless the measured
/// sizes grew since.
pub struct Baseline {
    known: HashSet<Key>,
    sizes: Sizes,
}

type Key = (PathBuf, WarningKind, Option<String>);

/// The fields of a saved JSON report that the baseline needs.
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
    lines: Lines,
    tokens: Tokens,
    #[serde(default)]
    depth: Depth,
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
    functions: HashMap<(PathBuf, String), [usize; 3]>,
    dirs: HashMap<PathBuf, [usize; 3]>,
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
        for f in files {
            sizes.files.insert(f.path, [f.tokens.0, f.lines.0, 0]);
        }
        for f in functions {
            sizes.add_function((f.path, f.name), [f.lines.0, f.tokens.0, f.depth.0]);
        }
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
            sizes
                .files
                .insert(f.path.clone(), [f.tokens.0, f.lines.0, 0]);
        }
        for f in &report.functions {
            sizes.add_function(
                (f.path.clone(), f.name.clone()),
                [f.lines.0, f.tokens.0, f.depth.0],
            );
        }
        sizes.add_dirs(&report.directories);
        sizes
    }

    /// Same-named functions in one file (overloads) keep the largest sizes.
    fn add_function(&mut self, key: (PathBuf, String), sizes: [usize; 3]) {
        let entry = self.functions.entry(key).or_default();
        for (e, s) in entry.iter_mut().zip(sizes) {
            *e = (*e).max(s);
        }
    }

    fn add_dirs(&mut self, dirs: &[DirStats]) {
        for d in dirs {
            self.dirs
                .insert(d.path.clone(), [d.files, d.dirs, d.tokens.0]);
        }
    }

    /// `None` for name-based warnings, which have no size.
    fn of(&self, w: &Warning) -> Option<[usize; 3]> {
        match w.kind {
            WarningKind::LargeFile => self.files.get(&w.path).copied(),
            WarningKind::LargeFunction | WarningKind::DeepFunction => {
                let name = w.subject.clone()?;
                self.functions.get(&(w.path.clone(), name)).copied()
            }
            WarningKind::FolderTooManyFiles
            | WarningKind::FolderTooManyDirs
            | WarningKind::FolderTooManyTokens => self.dirs.get(&w.path).copied(),
            WarningKind::FolderTooDeep
            | WarningKind::FolderTooFewEntries
            | WarningKind::FolderMissingRequired
            | WarningKind::FolderForbiddenName
            | WarningKind::FolderBadName
            | WarningKind::FolderSiblingModule
            | WarningKind::FolderUnexpectedDir => None,
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
    fn known_warnings_hide_until_they_grow() {
        let baseline = saved(
            r#"{"files":[{"path":"a.rs","tokens":100,"lines":10}],
                "functions":[{"path":"a.rs","name":"f","lines":50,"tokens":9}],
                "warnings":[{"path":"a.rs","kind":"large_file","message":""},
                            {"path":"a.rs","kind":"large_function","subject":"f","message":""}]}"#,
        );
        let mut current = Sizes::default();
        current.files.insert("a.rs".into(), [100, 10, 0]);
        current
            .functions
            .insert(("a.rs".into(), "f".into()), [51, 9, 0]);

        assert!(baseline.covers(&warning(WarningKind::LargeFile, None), &current));
        assert!(!baseline.covers(&warning(WarningKind::LargeFunction, Some("f")), &current));
        assert!(!baseline.covers(&warning(WarningKind::LargeFunction, Some("g")), &current));
    }
}
