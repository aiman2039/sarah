use std::collections::{HashMap, HashSet};
use std::path::Path;

use super::model::Report;
use super::warning::{Warning, WarningKind};
use crate::allow::{AllowTarget, TrackedAllow};
use crate::config::LimitRules;
use crate::count::FileStats;

/// Who `sarah-allow` comments cover: whole files, functions and types by their first line.
struct AllowIndex<'a> {
    files: HashMap<&'a Path, &'a [WarningKind]>,
    items: HashMap<&'a Path, Vec<(AllowTarget, &'a [WarningKind])>>,
}

impl<'a> AllowIndex<'a> {
    fn of(files: &'a [FileStats]) -> Self {
        let file_map = files
            .iter()
            .map(|f| (f.path.as_path(), f.allows.file.as_slice()))
            .collect();
        let items = files
            .iter()
            .map(|f| {
                let fns = f.functions.iter().flatten().map(|func| {
                    (
                        AllowTarget::Function(func.start_line),
                        func.allow.as_slice(),
                    )
                });
                let tys = f
                    .types
                    .iter()
                    .map(|ty| (AllowTarget::Type(ty.start_line), ty.allow.as_slice()));
                (f.path.as_path(), fns.chain(tys).collect())
            })
            .collect();
        Self {
            files: file_map,
            items,
        }
    }

    /// `true` when `w` is covered, recording the matching target(s) in `used`.
    fn check(&self, w: &Warning, used: &mut HashSet<(&'a Path, AllowTarget, WarningKind)>) -> bool {
        let mut covered = false;
        if let Some((&path, &kinds)) = self.files.get_key_value(w.path.as_path())
            && kinds.contains(&w.kind)
        {
            used.insert((path, AllowTarget::File, w.kind));
            covered = true;
        }
        if let Some(line) = w.line {
            let target = if w.kind == WarningKind::LargeType {
                AllowTarget::Type(line)
            } else {
                AllowTarget::Function(line)
            };
            if let Some((&path, entries)) = self.items.get_key_value(w.path.as_path())
                && entries
                    .iter()
                    .any(|(tgt, kinds)| *tgt == target && kinds.contains(&w.kind))
            {
                used.insert((path, target, w.kind));
                covered = true;
            }
        }
        covered
    }

    fn unused_warnings(
        files: &'a [FileStats],
        used: &HashSet<(&'a Path, AllowTarget, WarningKind)>,
        rules: &LimitRules,
    ) -> Vec<Warning> {
        let mut out = Vec::new();
        for file in files {
            if !rules.for_path(&file.path).unused_allow {
                continue;
            }
            for tracked in &file.allows.tracked {
                out.extend(Self::check_tracked(&file.path, tracked, used));
            }
        }
        out
    }

    fn check_tracked(
        path: &'a Path,
        tracked: &TrackedAllow,
        used: &HashSet<(&'a Path, AllowTarget, WarningKind)>,
    ) -> Vec<Warning> {
        if tracked.explicit {
            return tracked
                .kinds
                .iter()
                .filter(|kind| !used.contains(&(path, tracked.target, **kind)))
                .map(|kind| {
                    Warning::new(
                        path.to_path_buf(),
                        WarningKind::UnusedAllow,
                        format!(
                            "sarah-allow at L{}: `{kind}` was not triggered",
                            tracked.line
                        ),
                    )
                    .at_line(tracked.line)
                    .about(kind.to_string())
                })
                .collect();
        }
        if tracked
            .kinds
            .iter()
            .any(|k| used.contains(&(path, tracked.target, *k)))
        {
            return Vec::new();
        }
        vec![
            Warning::new(
                path.to_path_buf(),
                WarningKind::UnusedAllow,
                format!("sarah-allow at L{}: no warning was triggered", tracked.line),
            )
            .at_line(tracked.line),
        ]
    }
}

impl Report {
    /// Drop warnings that `sarah-allow` comments turn off, and emit `unused_allow` when enabled.
    pub fn apply_allows(&mut self, rules: &LimitRules) {
        let warnings = std::mem::take(&mut self.warnings);
        let before = warnings.len();
        let index = AllowIndex::of(&self.files);
        let mut used = HashSet::new();
        let mut kept: Vec<_> = warnings
            .into_iter()
            .filter(|w| !index.check(w, &mut used))
            .collect();
        self.allowed = before.saturating_sub(kept.len());
        kept.extend(AllowIndex::unused_warnings(&self.files, &used, rules));
        self.warnings = kept;
    }
}
