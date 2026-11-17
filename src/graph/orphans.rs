use std::path::Path;

use globset::GlobSet;

use super::resolve::{ImportGraph, RESOLVABLE_EXTS};
use super::{GraphHelpers, GraphRules};
use crate::count::FileStats;
use crate::report::{Warning, WarningKind};

impl GraphHelpers {
    /// `orphan_file` for resolvable code files that nothing imports and that are not entry points.
    pub(super) fn check_orphans(
        files: &[FileStats],
        graph: &ImportGraph<'_>,
        rules: &GraphRules,
    ) -> Vec<Warning> {
        let Some(entries) = rules.entry_points() else {
            return Vec::new();
        };
        let reached = Self::reached(files, graph);
        files
            .iter()
            .zip(reached)
            .filter(|(file, reached)| !reached && Self::can_be_orphan(&file.path, entries))
            .map(|(file, _)| {
                Warning::file(
                    file,
                    WarningKind::OrphanFile,
                    "no file imports this file and it is not an entry point — delete it, import it or add it to `entry_points`"
                        .to_string(),
                )
            })
            .collect()
    }

    fn can_be_orphan(path: &Path, entries: &GlobSet) -> bool {
        let resolvable = path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| RESOLVABLE_EXTS.contains(&e));
        resolvable && !entries.is_match(path)
    }

    /// Files reached by an import, a Rust `mod` declaration, or (Python) an import of
    /// their package's `__init__.py`, since `from pkg import mod` resolves to the package.
    fn reached(files: &[FileStats], graph: &ImportGraph<'_>) -> Vec<bool> {
        let mut reached = vec![false; files.len()];
        let imported = (0..graph.len()).flat_map(|from| graph.targets(from));
        for idx in imported.chain(graph.declared.iter().flatten().copied()) {
            if let Some(slot) = reached.get_mut(idx) {
                *slot = true;
            }
        }
        let by_package: Vec<bool> = files
            .iter()
            .map(|f| Self::package_reached(&f.path, graph, &reached))
            .collect();
        reached
            .into_iter()
            .zip(by_package)
            .map(|(a, b)| a || b)
            .collect()
    }

    fn package_reached(path: &Path, graph: &ImportGraph<'_>, reached: &[bool]) -> bool {
        if path.extension().is_none_or(|e| e != "py") {
            return false;
        }
        let init = path
            .parent()
            .unwrap_or_else(|| Path::new(""))
            .join("__init__.py");
        graph
            .index
            .get(init.as_path())
            .is_some_and(|&idx| reached.get(idx).copied().unwrap_or(false))
    }
}
