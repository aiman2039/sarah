use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};

use crate::count::FileStats;
use crate::functions::ImportHelpers;

const PROBE_EXTS: &[&str] = &[
    "",
    ".rs",
    "/mod.rs",
    ".ts",
    ".tsx",
    "/index.ts",
    "/index.tsx",
    ".js",
    ".jsx",
    "/index.js",
    ".py",
    "/__init__.py",
    ".go",
    ".h",
    ".hpp",
    ".zig",
    ".rb",
    ".php",
];

/// Resolves per-file `ImportRef`s to internal repository file indices.
pub(super) struct GraphResolver;

impl GraphResolver {
    pub(super) fn build_edges(files: &[FileStats]) -> Vec<Vec<usize>> {
        let index: HashMap<PathBuf, usize> = files
            .iter()
            .enumerate()
            .map(|(idx, f)| (f.path.clone(), idx))
            .collect();
        files
            .iter()
            .enumerate()
            .map(|(idx, file)| Self::file_edges(idx, file, &index))
            .collect()
    }

    fn file_edges(idx: usize, file: &FileStats, index: &HashMap<PathBuf, usize>) -> Vec<usize> {
        let mut targets = BTreeSet::new();
        for imp in &file.imports {
            if let Some(target) = Self::resolve_one(&file.path, &imp.path, index)
                && target != idx
            {
                targets.insert(target);
            }
        }
        targets.into_iter().collect()
    }

    fn resolve_one(from_file: &Path, raw: &str, index: &HashMap<PathBuf, usize>) -> Option<usize> {
        let normalized =
            ImportHelpers::resolve_relative(from_file, raw).unwrap_or_else(|| raw.to_string());
        if let Some(rest) = normalized.strip_prefix("crate::") {
            return Self::probe_rust_crate(rest, index);
        }
        let slash = normalized.replace("::", "/").replace('.', "/");
        Self::probe_candidates(&slash, index).or_else(|| Self::probe_candidates(&normalized, index))
    }

    fn probe_rust_crate(rest: &str, index: &HashMap<PathBuf, usize>) -> Option<usize> {
        let parts: Vec<&str> = rest
            .split("::")
            .filter(|s| !s.is_empty() && *s != "*")
            .collect();
        for len in (1..=parts.len()).rev() {
            let Some(slice) = parts.get(..len) else {
                continue;
            };
            let joined = slice.join("/");
            for prefix in ["src/", ""] {
                let base = format!("{prefix}{joined}");
                if let Some(hit) = Self::probe_candidates(&base, index) {
                    return Some(hit);
                }
            }
        }
        None
    }

    fn probe_candidates(base: &str, index: &HashMap<PathBuf, usize>) -> Option<usize> {
        let trimmed = base.trim_matches('/');
        if trimmed.is_empty() {
            return None;
        }
        for ext in PROBE_EXTS {
            let candidate = PathBuf::from(format!("{trimmed}{ext}"));
            if let Some(&idx) = index.get(&candidate) {
                return Some(idx);
            }
        }
        None
    }
}
