use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::count::FileStats;
use crate::units::{Depth, Lines, Tokens};

/// Totals for one folder. The analyzed root is `.`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DirStats {
    pub path: PathBuf,
    /// Levels below the root (the root is 0).
    pub depth: Depth,
    /// Files directly in the folder.
    pub files: usize,
    /// Folders directly in the folder.
    pub dirs: usize,
    /// Files in the whole subtree.
    pub total_files: usize,
    /// Lines in the whole subtree.
    pub lines: Lines,
    /// Tokens in the whole subtree.
    pub tokens: Tokens,
}

pub struct DirNode {
    pub stats: DirStats,
    pub file_names: Vec<String>,
    pub dir_names: BTreeSet<String>,
}

/// Folders that hold at least one analyzed file, keyed by path relative to the root.
pub struct DirTree {
    nodes: BTreeMap<PathBuf, DirNode>,
}

impl DirTree {
    pub fn build(files: &[FileStats]) -> Self {
        let mut tree = Self {
            nodes: BTreeMap::new(),
        };
        tree.node(Path::new(""));
        for f in files {
            tree.add_file(f);
        }
        tree.count_children();
        tree
    }

    pub fn nodes(&self) -> impl Iterator<Item = &DirNode> {
        self.nodes.values()
    }

    pub fn stats(&self) -> Vec<DirStats> {
        self.nodes.values().map(|n| n.stats.clone()).collect()
    }

    fn add_file(&mut self, f: &FileStats) {
        let parent = f.path.parent().unwrap_or(Path::new(""));
        if let Some(name) = f.path.file_name() {
            let name = name.to_string_lossy().into_owned();
            self.node(parent).file_names.push(name);
        }
        let mut child: Option<&Path> = None;
        for dir in parent.ancestors() {
            let node = self.node(dir);
            node.stats.total_files = node.stats.total_files.saturating_add(1);
            node.stats.lines = node.stats.lines.saturating_add(f.lines);
            node.stats.tokens = node.stats.tokens.saturating_add(f.tokens);
            if let Some(name) = child.and_then(Path::file_name) {
                node.dir_names.insert(name.to_string_lossy().into_owned());
            }
            child = Some(dir);
        }
    }

    fn count_children(&mut self) {
        for node in self.nodes.values_mut() {
            node.stats.files = node.file_names.len();
            node.stats.dirs = node.dir_names.len();
        }
    }

    /// The node for `dir` (`""` is the root), created on first use.
    fn node(&mut self, dir: &Path) -> &mut DirNode {
        let key = if dir.as_os_str().is_empty() {
            PathBuf::from(".")
        } else {
            dir.to_path_buf()
        };
        let depth = Depth(dir.components().count());
        self.nodes.entry(key.clone()).or_insert_with(|| DirNode {
            stats: DirStats {
                path: key,
                depth,
                files: 0,
                dirs: 0,
                total_files: 0,
                lines: Lines(0),
                tokens: Tokens(0),
            },
            file_names: Vec::new(),
            dir_names: BTreeSet::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::units::Bytes;

    fn file(path: &str, tokens: usize) -> FileStats {
        FileStats {
            path: path.into(),
            language: "Rust".into(),
            bytes: Bytes(0),
            lines: Lines(1),
            code: Lines(1),
            comments: Lines(0),
            blanks: Lines(0),
            tokens: Tokens(tokens),
            functions: None,
        }
    }

    #[test]
    fn builds_totals_and_children() {
        let tree = DirTree::build(&[
            file("main.rs", 1),
            file("src/a.rs", 10),
            file("src/x/b.rs", 100),
            file("src/x/y/c.rs", 1000),
        ]);
        let stats: Vec<_> = tree
            .stats()
            .into_iter()
            .map(|s| {
                (
                    s.path,
                    s.depth.0,
                    s.files,
                    s.dirs,
                    s.total_files,
                    s.tokens.0,
                )
            })
            .collect();
        assert_eq!(
            stats,
            vec![
                (".".into(), 0, 1, 1, 4, 1111),
                ("src".into(), 1, 1, 1, 3, 1110),
                ("src/x".into(), 2, 1, 1, 2, 1100),
                ("src/x/y".into(), 3, 1, 0, 1, 1000),
            ]
        );
    }
}
