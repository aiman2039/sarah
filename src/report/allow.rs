use std::collections::{HashMap, HashSet};
use std::path::Path;

use super::model::{Report, Warning, WarningKind};
use crate::units::LineNo;

/// Who `sarah-allow` comments cover: whole files, and functions by their first line.
struct AllowIndex<'a> {
    files: HashMap<&'a Path, &'a [WarningKind]>,
    functions: HashSet<(&'a Path, LineNo, WarningKind)>,
}

impl<'a> AllowIndex<'a> {
    fn of(report: &'a Report) -> Self {
        let files = report
            .files
            .iter()
            .map(|f| (f.path.as_path(), f.allows.file.as_slice()))
            .collect();
        let functions = report
            .functions
            .iter()
            .flat_map(|func| {
                func.allow
                    .iter()
                    .map(|kind| (func.path.as_path(), func.start_line, *kind))
            })
            .collect();
        Self { files, functions }
    }

    fn covers(&self, w: &Warning) -> bool {
        let file = self
            .files
            .get(w.path.as_path())
            .is_some_and(|kinds| kinds.contains(&w.kind));
        let function = w
            .line
            .is_some_and(|line| self.functions.contains(&(w.path.as_path(), line, w.kind)));
        file || function
    }
}

impl Report {
    /// Drop warnings that `sarah-allow` comments turn off.
    pub fn apply_allows(&mut self) {
        let warnings = std::mem::take(&mut self.warnings);
        let before = warnings.len();
        let index = AllowIndex::of(self);
        let kept: Vec<_> = warnings.into_iter().filter(|w| !index.covers(w)).collect();
        self.allowed = before.saturating_sub(kept.len());
        self.warnings = kept;
    }
}
