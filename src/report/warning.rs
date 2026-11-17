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
    TooManyImports,
    LargeFunction,
    DeepFunction,
    TooManyParams,
    TooManyBoolParams,
    ComplexFunction,
    LargeType,
    ForbiddenImport,
    CircularImport,
    DuplicateCode,
    TodoComment,
    LayerViolation,
    HighFanIn,
    HighFanOut,
    OrphanFile,
    DisallowedImport,
    LargeContext,
    MissingTest,
    ForbiddenPattern,
    FolderTooManyFiles,
    FolderTooManyTotalFiles,
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
    FolderLowTestRatio,
    FolderHighTestRatio,
    BadAllow,
    UnusedAllow,
    TooManyReturns,
    TooManyTypes,
    LongLine,
    LowCommentRatio,
    HighCommentRatio,
    TooManyPublicItems,
}

impl WarningKind {
    /// Folder-structure warnings (as opposed to file and function warnings).
    pub fn is_folder(self) -> bool {
        self.to_string().starts_with("folder_")
    }

    /// Kinds that need the whole walked tree (folder rules, `missing_test`); a single-file
    /// check leaves them out.
    pub fn needs_whole_tree(self) -> bool {
        self.is_folder() || self == Self::MissingTest
    }

    /// Human-readable rule summary for SARIF code scanning.
    pub fn description(self) -> &'static str {
        self.folder_description()
            .or_else(|| self.function_description())
            .or_else(|| self.import_description())
            .unwrap_or_else(|| self.file_description())
    }

    fn folder_description(self) -> Option<&'static str> {
        Some(match self {
            Self::FolderTooManyFiles => "Folder has too many direct files",
            Self::FolderTooManyTotalFiles => "Folder subtree has too many total files",
            Self::FolderTooManyDirs => "Folder has too many direct sub-folders",
            Self::FolderTooDeep => "Folder hierarchy exceeds maximum nesting depth",
            Self::FolderTooManyLines => "Folder subtree exceeds line limit",
            Self::FolderTooManyTokens => "Folder subtree exceeds token limit",
            Self::FolderTooFewEntries => "Folder has fewer entries than minimum",
            Self::FolderMissingRequired => "Folder is missing a required file",
            Self::FolderForbiddenName => "File or folder matches a forbidden name pattern",
            Self::FolderBadName => "File or folder violates naming convention",
            Self::FolderSiblingModule => "File shares a name with a sibling folder",
            Self::FolderUnexpectedDir => "Sub-folder is not in the allow_only list",
            Self::FolderLowTestRatio => "Folder subtree has too little test code",
            Self::FolderHighTestRatio => "Folder subtree has too much test code",
            _ => return None,
        })
    }

    fn function_description(self) -> Option<&'static str> {
        Some(match self {
            Self::LargeFunction => "Function exceeds line or token limit",
            Self::DeepFunction => "Function control-flow nesting is too deep",
            Self::TooManyParams => "Function has too many parameters",
            Self::TooManyBoolParams => "Function has too many boolean flag parameters",
            Self::ComplexFunction => "Function exceeds cyclomatic or cognitive complexity limit",
            Self::TooManyReturns => "Function has too many return statements",
            Self::LargeType => "Struct or class has too many fields or methods",
            _ => return None,
        })
    }

    fn import_description(self) -> Option<&'static str> {
        Some(match self {
            Self::ForbiddenImport => "Import matches a forbidden module pattern",
            Self::CircularImport => "Files form a circular import dependency",
            Self::LayerViolation => "File imports a file of a higher layer",
            Self::HighFanIn => "Too many files import this file",
            Self::HighFanOut => "File imports too many internal files",
            Self::OrphanFile => "No file imports this file and it is not an entry point",
            Self::DisallowedImport => "Import matches no allowed module pattern",
            Self::LargeContext => "File and its transitive imports exceed the context token limit",
            _ => return None,
        })
    }

    fn file_description(self) -> &'static str {
        match self {
            Self::LargeFile => "File exceeds token or line limit",
            Self::TooManyFunctions => "File declares too many functions",
            Self::TooManyImports => "File imports too many modules",
            Self::DuplicateCode => "Duplicate code block appears in multiple places",
            Self::TodoComment => "Comment contains a forbidden marker or exceeds TODO limit",
            Self::MissingTest => "File has no matching test file and no inline test",
            Self::ForbiddenPattern => "Line matches a forbidden pattern",
            Self::BadAllow => "Malformed or misplaced sarah-allow directive",
            Self::TooManyTypes => "File declares too many types",
            Self::LongLine => "File has lines over the length limit",
            Self::LowCommentRatio => "File has too few comment lines for its code",
            Self::HighCommentRatio => "File has too many comment lines for its code",
            Self::TooManyPublicItems => "File exposes too many public items",
            _ => "sarah-allow directive suppressed no warning",
        }
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
