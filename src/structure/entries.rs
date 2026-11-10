use std::path::Path;

use super::rules::Policy;
use super::tree::DirNode;
use crate::functions::AstHelpers;
use crate::report::{Warning, WarningKind};

/// Entry-name and module-layout checks for one folder (`require`, `forbid`, `naming`, etc.).
pub(super) struct EntryCheck<'a> {
    pub node: &'a DirNode,
    pub policy: &'a Policy<'a>,
}

impl EntryCheck<'_> {
    pub(super) fn warnings(&self) -> Vec<Warning> {
        self.missing_required()
            .into_iter()
            .chain(self.forbidden_names())
            .chain(self.bad_names())
            .chain(self.sibling_modules())
            .chain(self.unexpected_dirs())
            .collect()
    }

    fn dir(&self) -> &Path {
        &self.node.stats.path
    }

    fn entry(&self, name: &str, kind: WarningKind, message: String) -> Warning {
        let path = if self.dir() == Path::new(".") {
            name.into()
        } else {
            self.dir().join(name)
        };
        Warning::new(path, kind, message)
    }

    fn missing_required(&self) -> Vec<Warning> {
        let Some(require) = self.policy.require else {
            return Vec::new();
        };
        require
            .iter()
            .filter(|(_, m)| !self.node.file_names.iter().any(|n| m.is_match(n)))
            .map(|(pattern, _)| {
                Warning::new(
                    self.dir().to_path_buf(),
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
        self.node
            .file_names
            .iter()
            .chain(&self.node.dir_names)
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
