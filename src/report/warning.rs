use std::path::PathBuf;

use derive_more::Display;
use serde::{Deserialize, Serialize};

use crate::units::LineNo;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Display, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[display(rename_all = "snake_case")]
pub enum WarningKind {
    LargeFile,
    TooManyFunctions,
    LargeFunction,
    DeepFunction,
    /// More parameters than `max_fn_params`.
    TooManyParams,
    /// More boolean parameters than `max_fn_bool_params`.
    TooManyBoolParams,
    /// Over `max_fn_cyclomatic` or `max_fn_cognitive`.
    ComplexFunction,
    /// Over `max_type_fields` or `max_type_methods`.
    LargeType,
    /// Import matches `forbid_imports`.
    ForbiddenImport,
    FolderTooManyFiles,
    FolderTooManyDirs,
    FolderTooDeep,
    FolderTooManyLines,
    FolderTooManyTokens,
    FolderTooFewEntries,
    FolderMissingRequired,
    FolderForbiddenName,
    FolderBadName,
    FolderSiblingModule,
    FolderUnexpectedDir,
    /// A malformed or misplaced `sarah-allow` comment.
    BadAllow,
    /// A `sarah-allow` comment that suppressed no warning (when `unused_allow` is on).
    UnusedAllow,
}

impl WarningKind {
    /// Folder-structure warnings (as opposed to file and function warnings).
    pub fn is_folder(self) -> bool {
        self.to_string().starts_with("folder_")
    }
}

#[derive(Debug, Serialize)]
pub struct Warning {
    pub path: PathBuf,
    pub kind: WarningKind,
    pub message: String,
    /// First line of the offending item, when there is one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<LineNo>,
    /// What the warning is about inside `path` (a function/type name, an import, a glob);
    /// with `path` and `kind` it identifies the warning across runs (`--baseline`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,
}

impl Warning {
    pub fn new(path: PathBuf, kind: WarningKind, message: String) -> Self {
        Self {
            path,
            kind,
            message,
            line: None,
            subject: None,
        }
    }

    pub fn at_line(self, line: LineNo) -> Self {
        Self {
            line: Some(line),
            ..self
        }
    }

    pub fn about(self, subject: String) -> Self {
        Self {
            subject: Some(subject),
            ..self
        }
    }
}
