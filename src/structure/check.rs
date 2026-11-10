use std::path::Path;

use super::rules::{FolderRules, Policy};
use super::tree::{DirNode, DirTree};
use crate::functions::AstHelpers;
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

    /// Direct-entry violations for one folder (used by single-file MCP checks).
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
            self.too_many_dirs(),
            self.too_deep(),
            with_subtree.then(|| self.too_many_lines()).flatten(),
            with_subtree.then(|| self.too_many_tokens()).flatten(),
            self.too_few_entries(),
        ]
        .into_iter()
        .flatten()
        .chain(self.missing_required())
        .chain(self.forbidden_names())
        .chain(self.bad_names())
        .chain(self.sibling_modules())
        .chain(self.unexpected_dirs())
        .collect()
    }

    fn dir(&self) -> &Path {
        &self.node.stats.path
    }

    fn folder(&self, kind: WarningKind, message: String) -> Warning {
        Warning::new(self.dir().to_path_buf(), kind, message)
    }

    fn entry(&self, name: &str, kind: WarningKind, message: String) -> Warning {
        let path = if self.dir() == Path::new(".") {
            name.into()
        } else {
            self.dir().join(name)
        };
        Warning::new(path, kind, message)
    }

    fn all_names(&self) -> impl Iterator<Item = &String> {
        self.node.file_names.iter().chain(&self.node.dir_names)
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
        let max = self.policy.max_depth?;
        let depth = self.node.stats.depth;
        (depth.0 == max.0.saturating_add(1)).then(|| {
            self.folder(
                WarningKind::FolderTooDeep,
                format!("depth {depth} (>{max}) — flatten the hierarchy"),
            )
        })
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

    fn missing_required(&self) -> Vec<Warning> {
        let Some(require) = self.policy.require else {
            return Vec::new();
        };
        require
            .iter()
            .filter(|(_, m)| !self.node.file_names.iter().any(|n| m.is_match(n)))
            .map(|(pattern, _)| {
                self.folder(
                    WarningKind::FolderMissingRequired,
                    format!("no file matches required {pattern:?}"),
                )
                .about(pattern.to_string())
            })
            .collect()
    }

    fn forbidden_names(&self) -> Vec<Warning> {
        let Some(forbid) = self.policy.forbid else {
            return Vec::new();
        };
        self.all_names()
            .filter_map(|name| {
                let pattern = forbid.find(name)?;
                Some(self.entry(
                    name,
                    WarningKind::FolderForbiddenName,
                    format!("name matches forbidden {pattern:?} — name it after what it does"),
                ))
            })
            .collect()
    }

    fn bad_names(&self) -> Vec<Warning> {
        let Some(naming) = self.policy.naming else {
            return Vec::new();
        };
        let bad_files = self
            .node
            .file_names
            .iter()
            .filter(|name| !naming.accepts(name));
        let bad_dirs = self
            .node
            .dir_names
            .iter()
            .filter(|name| !naming.accepts_dir(name));
        bad_files
            .chain(bad_dirs)
            .map(|name| {
                self.entry(
                    name,
                    WarningKind::FolderBadName,
                    format!("name is not {naming}"),
                )
            })
            .collect()
    }

    fn sibling_modules(&self) -> Vec<Warning> {
        if !self.policy.no_sibling_module {
            return Vec::new();
        }
        self.node
            .file_names
            .iter()
            .filter(|file| {
                let path = Path::new(file.as_str());
                AstHelpers::is_code_file(path)
                    && path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .is_some_and(|stem| self.node.dir_names.contains(stem))
            })
            .map(|file| {
                self.entry(
                    file,
                    WarningKind::FolderSiblingModule,
                    Self::sibling_hint(file).to_string(),
                )
            })
            .collect()
    }

    fn sibling_hint(file: &str) -> &'static str {
        match Path::new(file).extension().and_then(|e| e.to_str()) {
            Some("rs") => {
                "a folder with the same name exists — keep one module root (folder/mod.rs)"
            }
            Some("py" | "pyi") => {
                "a folder with the same name exists — keep one module root (folder/__init__.py)"
            }
            Some("ts" | "tsx" | "js" | "jsx" | "mjs" | "cjs" | "mts" | "cts") => {
                "a folder with the same name exists — keep one module root (folder/index.*)"
            }
            _ => "a folder with the same name exists — keep one module root",
        }
    }

    fn unexpected_dirs(&self) -> Vec<Warning> {
        let Some(allow_only) = self.policy.allow_only else {
            return Vec::new();
        };
        self.node
            .dir_names
            .iter()
            .filter(|name| allow_only.find(name).is_none())
            .map(|name| {
                self.entry(
                    name,
                    WarningKind::FolderUnexpectedDir,
                    "folder is not in allow_only".to_string(),
                )
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::allow::Allows;
    use crate::count::FileStats;
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
            allows: Allows::default(),
        }
    }

    fn check(rule: FolderRule, files: &[&str]) -> Vec<(PathBuf, WarningKind)> {
        let files: Vec<_> = files.iter().map(|p| file(p)).collect();
        FolderRules::new(None, vec![rule])
            .unwrap()
            .check(&DirTree::build(&files))
            .into_iter()
            .map(|w| (w.path, w.kind))
            .collect()
    }

    #[test]
    fn every_rule_fires_and_non_module_siblings_pass() {
        let got = check(
            FolderRule {
                path: "src/**".into(),
                max_files: Some(2),
                max_dirs: Some(0),
                max_depth: Some(Depth(2)),
                max_lines: Some(Lines(25)),
                max_tokens: Some(Tokens(25)),
                min_files: Some(2),
                require: Some(vec!["mod.rs".into()]),
                forbid: Some(vec!["utils*".into()]),
                allow_only: Some(vec!["core".into()]),
                naming: Some(Naming::Snake),
                no_sibling_module: Some(true),
                ..FolderRule::default()
            },
            &[
                "src/main.rs",
                "src/a/mod.rs",
                "src/a/utils.rs",
                "src/a/Bad.rs",
                "src/a/core.rs",
                "src/a/core.test.rs",
                "src/a/core.md",
                "src/a/core/mod.rs",
                "src/a/core/deep/x.rs",
            ],
        );
        assert!(got.contains(&("src/a".into(), WarningKind::FolderTooManyLines)));
        assert!(got.contains(&("src/a/core.rs".into(), WarningKind::FolderSiblingModule)));
        assert!(!got.contains(&(
            "src/a/core.test.rs".into(),
            WarningKind::FolderSiblingModule
        )));
        assert!(!got.contains(&("src/a/core.md".into(), WarningKind::FolderSiblingModule)));
    }
}
