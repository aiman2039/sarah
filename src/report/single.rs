use std::path::Path;

use super::model::{Report, ReportInput, SortBy};
use super::warning::Warning;
use crate::config::LimitRules;
use crate::count::FileStats;
use crate::graph::GraphRules;
use crate::structure::{DirNode, FolderRules};
use crate::walk::Skipped;

/// One file checked without a walk (LSP, MCP `check_file`), with its parent folder.
pub struct SingleFileInput {
    pub file: FileStats,
    pub sort: SortBy,
    pub limits: LimitRules,
    pub folders: FolderRules,
    pub tokenizer: String,
    /// The folder that holds `file`, read from disk; `None` skips the folder rules.
    pub parent: Option<DirNode>,
}

impl Report {
    /// The report of one file: rules that need the whole tree are off, and the folder
    /// rules run on the parent folder alone.
    pub fn single_file(input: SingleFileInput) -> Self {
        let SingleFileInput {
            file,
            sort,
            limits,
            folders,
            tokenizer,
            parent,
        } = input;
        let folder_warnings = Self::parent_warnings(&folders, parent.as_ref(), &file.path);
        let mut report = Self::build(ReportInput {
            files: vec![file],
            skipped: Skipped::default(),
            sort,
            limits,
            folders,
            tokenizer,
            no_cycles: false,
            max_duplicate_lines: None,
            graph: GraphRules::default(),
        });
        report.warnings.retain(|w| !w.kind.needs_whole_tree());
        report.warnings.extend(folder_warnings);
        report
    }

    /// Folder warnings of `parent` about `file` or `parent` itself.
    fn parent_warnings(
        folders: &FolderRules,
        parent: Option<&DirNode>,
        file: &Path,
    ) -> Vec<Warning> {
        let Some(node) = parent else {
            return Vec::new();
        };
        let dir = &node.stats.path;
        folders
            .check_single_dir(node)
            .into_iter()
            .filter(|w| w.path == file || &w.path == dir)
            .collect()
    }
}
