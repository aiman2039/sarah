use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

use crate::count::FileStats;
use crate::functions::ImportHelpers;
use crate::units::LineNo;

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

/// Extensions whose imports the resolver maps to files; other languages are never orphans.
pub(super) const RESOLVABLE_EXTS: &[&str] = &["rs", "py", "js", "jsx", "ts", "tsx"];

/// Resolves per-file `ImportRef`s to internal repository file indices.
struct GraphResolver;

/// Resolved internal edges, built once and shared by every graph check.
pub(super) struct ImportGraph<'a> {
    /// `edges[i]`: the distinct files that file `i` imports (no self-edges), each with the
    /// line of its first import.
    edges: Vec<BTreeMap<usize, LineNo>>,
    /// `declared[i]`: the files that Rust `mod name;` declarations in file `i` name.
    pub declared: Vec<Vec<usize>>,
    /// File index by path.
    pub index: HashMap<&'a Path, usize>,
}

impl<'a> ImportGraph<'a> {
    pub(super) fn build(files: &'a [FileStats]) -> Self {
        let index: HashMap<&Path, usize> = files
            .iter()
            .enumerate()
            .map(|(idx, f)| (f.path.as_path(), idx))
            .collect();
        let edges = files
            .iter()
            .enumerate()
            .map(|(idx, file)| GraphResolver::file_edges(idx, file, &index))
            .collect();
        let declared = files
            .iter()
            .map(|file| GraphResolver::declared_modules(file, &index))
            .collect();
        Self {
            edges,
            declared,
            index,
        }
    }

    /// Number of files.
    pub(super) fn len(&self) -> usize {
        self.edges.len()
    }

    /// The files that `from` imports, in index order.
    pub(super) fn targets(&self, from: usize) -> impl Iterator<Item = usize> + '_ {
        self.edges
            .get(from)
            .into_iter()
            .flat_map(|e| e.keys().copied())
    }

    /// Number of distinct files that `from` imports.
    pub(super) fn fan_out(&self, from: usize) -> usize {
        self.edges.get(from).map_or(0, BTreeMap::len)
    }

    pub(super) fn has_edge(&self, from: usize, to: usize) -> bool {
        self.edges.get(from).is_some_and(|e| e.contains_key(&to))
    }

    /// First import line of the edge `from -> to`.
    pub(super) fn line(&self, from: usize, to: usize) -> Option<LineNo> {
        self.edges.get(from)?.get(&to).copied()
    }
}

impl GraphResolver {
    fn file_edges(
        idx: usize,
        file: &FileStats,
        index: &HashMap<&Path, usize>,
    ) -> BTreeMap<usize, LineNo> {
        let mut targets = BTreeMap::new();
        for imp in &file.imports {
            if let Some(target) = Self::resolve_one(&file.path, &imp.path, index)
                && target != idx
            {
                targets.entry(target).or_insert(imp.line);
            }
        }
        targets
    }

    /// Files that `mod name;` in `file` names: next to it (a `mod.rs` or crate root) or in
    /// the folder named after it. Both are probed, so either layout counts as in use.
    fn declared_modules(file: &FileStats, index: &HashMap<&Path, usize>) -> Vec<usize> {
        let dir = file.path.parent().unwrap_or_else(|| Path::new(""));
        let nested = file.path.file_stem().map(|stem| dir.join(stem));
        let bases: Vec<PathBuf> = nested.into_iter().chain([dir.to_path_buf()]).collect();
        file.module_decls
            .iter()
            .flat_map(|name| bases.iter().map(move |base| base.join(name)))
            .filter_map(|base| Self::probe_candidates(base.to_str()?, index))
            .collect()
    }

    fn resolve_one(from_file: &Path, raw: &str, index: &HashMap<&Path, usize>) -> Option<usize> {
        let normalized =
            ImportHelpers::resolve_relative(from_file, raw).unwrap_or_else(|| raw.to_string());
        if let Some(rest) = normalized.strip_prefix("crate::") {
            return Self::probe_rust_crate(rest, index);
        }
        let slash = normalized.replace("::", "/").replace('.', "/");
        Self::probe_candidates(&slash, index).or_else(|| Self::probe_candidates(&normalized, index))
    }

    fn probe_rust_crate(rest: &str, index: &HashMap<&Path, usize>) -> Option<usize> {
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

    fn probe_candidates(base: &str, index: &HashMap<&Path, usize>) -> Option<usize> {
        let trimmed = base.trim_matches('/');
        if trimmed.is_empty() {
            return None;
        }
        for ext in PROBE_EXTS {
            let candidate = PathBuf::from(format!("{trimmed}{ext}"));
            if let Some(&idx) = index.get(candidate.as_path()) {
                return Some(idx);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AnalysisRules;
    use crate::count::{Analysis, AnalysisRun, SourceInput, Tokenizer};

    fn rust(path: &str, text: &str) -> FileStats {
        let Analysis::Counted(stats) = FileStats::analyze_source(SourceInput {
            path: PathBuf::from(path),
            text,
            run: AnalysisRun {
                tokenizer: Tokenizer::O200k,
                with_functions: true,
                skip_generated: false,
            },
            rules: AnalysisRules::default(),
        }) else {
            panic!("{path} not counted");
        };
        *stats
    }

    #[test]
    fn mod_declarations_resolve_to_files_but_are_not_imports() {
        let files = [
            rust(
                "src/main.rs",
                "pub(crate) mod a;\n#[cfg(test)]\nmod r#b;\nmod inline {}\n",
            ),
            rust("src/a/mod.rs", "mod c;\n"),
            rust("src/b.rs", ""),
            rust("src/a/c.rs", ""),
            rust("src/inline.rs", ""),
        ];
        let graph = ImportGraph::build(&files);
        assert_eq!(
            graph.declared,
            vec![vec![1, 2], vec![3], vec![], vec![], vec![]]
        );
        assert_eq!(graph.fan_out(0), 0);
    }
}
