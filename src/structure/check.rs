use std::path::{Component, Path};

use super::entries::EntryCheck;
use super::rules::{FolderRules, Policy};
use super::tree::{DirNode, DirTree};
use crate::report::{Warning, WarningKind};

impl FolderRules {
    /// Every rule violation, folder by folder in path order.
    pub fn check(&self, tree: &DirTree) -> Vec<Warning> {
        tree.nodes()
            .filter_map(|node| {
                let policy = self.policy_for(&node.stats.path)?;
                Some(FolderCheck { node, policy }.warnings(true))
            })
            .flatten()
            .collect()
    }

    /// Direct-entry violations for one folder (used by single-file MCP and LSP checks).
    pub fn check_single_dir(&self, node: &DirNode) -> Vec<Warning> {
        let Some(policy) = self.policy_for(&node.stats.path) else {
            return Vec::new();
        };
        FolderCheck { node, policy }.warnings(false)
    }
}

struct FolderCheck<'a> {
    node: &'a DirNode,
    policy: Policy<'a>,
}

impl FolderCheck<'_> {
    fn warnings(&self, with_subtree: bool) -> Vec<Warning> {
        [
            self.too_many_files(),
            with_subtree.then(|| self.too_many_total_files()).flatten(),
            self.too_many_dirs(),
            self.too_deep(),
            with_subtree.then(|| self.too_many_lines()).flatten(),
            with_subtree.then(|| self.too_many_tokens()).flatten(),
            self.too_few_entries(),
        ]
        .into_iter()
        .flatten()
        .chain(
            EntryCheck {
                node: self.node,
                policy: &self.policy,
            }
            .warnings(),
        )
        .collect()
    }

    fn folder(&self, kind: WarningKind, message: String) -> Warning {
        Warning::new(self.node.stats.path.clone(), kind, message)
    }

    fn too_many_files(&self) -> Option<Warning> {
        let max = self.policy.max_files?;
        let count = self.node.stats.files;
        (count > max).then(|| {
            self.folder(
                WarningKind::FolderTooManyFiles,
                format!("{count} files (>{max}) — group them into sub-folders by topic"),
            )
        })
    }

    fn too_many_total_files(&self) -> Option<Warning> {
        let max = self.policy.max_total_files?;
        let count = self.node.stats.total_files;
        (count > max).then(|| {
            self.folder(
                WarningKind::FolderTooManyTotalFiles,
                format!("{count} total files in subtree (>{max}) — split into top-level domains"),
            )
        })
    }

    fn too_many_dirs(&self) -> Option<Warning> {
        let max = self.policy.max_dirs?;
        let count = self.node.stats.dirs;
        (count > max).then(|| {
            self.folder(
                WarningKind::FolderTooManyDirs,
                format!("{count} sub-folders (>{max}) — merge or regroup related topics"),
            )
        })
    }

    fn too_deep(&self) -> Option<Warning> {
        if let Some(max) = self.policy.max_depth {
            let depth = self.node.stats.depth;
            if depth.0 == max.0.saturating_add(1) {
                return Some(self.folder(
                    WarningKind::FolderTooDeep,
                    format!("depth {depth} (>{max}) — flatten the hierarchy"),
                ));
            }
        }
        let (max_rel, anchor) = self.policy.max_relative_depth?;
        let rel = Self::relative_depth(&self.node.stats.path, anchor);
        (rel == max_rel.0.saturating_add(1)).then(|| {
            let label = anchor.map_or(String::new(), |a| format!(" below {a}/"));
            self.folder(
                WarningKind::FolderTooDeep,
                format!("depth {rel}{label} (>{max_rel}) — flatten the hierarchy"),
            )
        })
    }

    fn relative_depth(path: &Path, anchor: Option<&str>) -> usize {
        if path == Path::new(".") {
            return 0;
        }
        let comps: Vec<_> = path
            .components()
            .filter_map(|c| match c {
                Component::Normal(s) => s.to_str(),
                _ => None,
            })
            .collect();
        let Some(anchor) = anchor else {
            return comps.len();
        };
        let Some(pos) = comps.iter().rposition(|c| *c == anchor) else {
            return comps.len();
        };
        comps.len().saturating_sub(pos.saturating_add(1))
    }

    fn too_many_lines(&self) -> Option<Warning> {
        let max = self.policy.max_lines?;
        let stats = &self.node.stats;
        (stats.lines > max).then(|| {
            self.folder(
                WarningKind::FolderTooManyLines,
                format!(
                    "{} lines in {} files (>{max}) — split the folder by topic",
                    stats.lines, stats.total_files
                ),
            )
        })
    }

    fn too_many_tokens(&self) -> Option<Warning> {
        let max = self.policy.max_tokens?;
        let stats = &self.node.stats;
        (stats.tokens > max).then(|| {
            self.folder(
                WarningKind::FolderTooManyTokens,
                format!(
                    "{} tokens in {} files (>{max}) — too big for one agent context; split the topic",
                    stats.tokens, stats.total_files
                ),
            )
        })
    }

    fn too_few_entries(&self) -> Option<Warning> {
        let min = self.policy.min_files?;
        let count = self.node.stats.files.saturating_add(self.node.stats.dirs);
        (count < min).then(|| {
            self.folder(
                WarningKind::FolderTooFewEntries,
                format!("{count} entries (<{min}) — merge it into its parent"),
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::allow::Allows;
    use crate::count::{CommentInfo, FileStats};
    use crate::structure::FolderRule;
    use crate::structure::naming::Naming;
    use crate::units::{Bytes, Depth, Lines, Tokens};

    fn file(path: &str) -> FileStats {
        FileStats {
            path: path.into(),
            language: "Rust".into(),
            bytes: Bytes(0),
            lines: Lines(10),
            code: Lines(10),
            comments: Lines(0),
            blanks: Lines(0),
            tokens: Tokens(10),
            functions: None,
            types: Vec::new(),
            imports: Vec::new(),
            comment_info: CommentInfo::default(),
            norm_lines: Vec::new(),
            allows: Allows::default(),
        }
    }

    #[test]
    fn subtree_total_files_and_monorepo_relative_depth_fire() {
        let files: Vec<_> = ["apps/web/src/a/b/c/x.rs", "apps/web/src/a/b/y.rs"]
            .into_iter()
            .map(file)
            .collect();
        let rules = FolderRules::new(
            None,
            vec![FolderRule {
                path: "**/src/**".into(),
                max_total_files: Some(1),
                max_relative_depth: Some(Depth(2)),
                naming: Some(Naming::Snake),
                ..FolderRule::default()
            }],
        )
        .unwrap();
        let got: Vec<(PathBuf, WarningKind)> = rules
            .check(&DirTree::build(&files))
            .into_iter()
            .map(|w| (w.path, w.kind))
            .collect();
        assert!(got.contains(&(
            "apps/web/src/a".into(),
            WarningKind::FolderTooManyTotalFiles
        )));
        assert!(got.contains(&("apps/web/src/a/b/c".into(), WarningKind::FolderTooDeep)));
    }
}
