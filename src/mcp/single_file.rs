use std::collections::BTreeSet;
use std::path::Path;

use anyhow::{Result, bail};
use serde_json::json;

use crate::cache::ScanCache;
use crate::cli::CheckArgs;
use crate::config::Config;
use crate::count::{Analysis, AnalyzeInput, FileStats};
use crate::report::{Report, SingleFileInput};
use crate::structure::{DirNode, DirStats, FolderRules};
use crate::units::{Depth, Lines, Tokens};
use crate::walk::FileEntry;

/// One `check_file` call: a single file and its parent folder, checked without a walk.
pub(super) struct SingleFileCheck<'a> {
    pub check: &'a CheckArgs,
    pub config: &'a Config,
    pub cache: &'a ScanCache,
    pub root: &'a Path,
    pub real: &'a Path,
    pub relative: &'a Path,
}

impl SingleFileCheck<'_> {
    pub(super) fn execute(&self) -> Result<String> {
        let unanalyzed = || {
            format!(
                "{} was not analyzed (excluded, ignored, generated or not text)",
                self.relative.display()
            )
        };
        if !self.passes_walk()? {
            bail!("{}", unanalyzed());
        }
        let limits = self.check.limit_rules(self.config)?;
        let test_ratio = self.check.folder_rules(self.config)?.has_test_ratio();
        let Analysis::Counted(stats) = self.cache.analyze(AnalyzeInput {
            entry: FileEntry {
                real: self.real.to_path_buf(),
                display: self.relative.to_path_buf(),
            },
            run: self.check.analysis_run(),
            rules: limits.analysis_for(self.relative, test_ratio),
        }) else {
            bail!("{}", unanalyzed());
        };
        self.render_json(*stats)
    }

    fn render_json(&self, stats: FileStats) -> Result<String> {
        let limits = self.check.limit_rules(self.config)?;
        let mut report = Report::single_file(SingleFileInput {
            file: stats,
            sort: self.check.sort,
            limits: self.check.limit_rules(self.config)?,
            folders: self.check.folder_rules(self.config)?,
            tokenizer: self.check.walk.tokenizer.to_string(),
            parent: self.read_parent_dir(),
        });
        if !self.check.no_allow {
            report.apply_allows(&limits);
        }
        let Some(file_out) = report.files.first() else {
            bail!("missing file stats");
        };
        Ok(serde_json::to_string_pretty(&json!({
            "file": file_out,
            "warnings": report.warnings,
        }))?)
    }

    fn read_parent_dir(&self) -> Option<DirNode> {
        let parent_real = self.real.parent()?;
        let parent_rel = FolderRules::folder_of(self.relative).to_path_buf();
        let depth = if parent_rel == Path::new(".") {
            Depth(0)
        } else {
            Depth(parent_rel.components().count())
        };
        let (file_names, dir_names) = self.scan_dir_entries(parent_real)?;
        Some(DirNode {
            stats: DirStats {
                path: parent_rel,
                depth,
                files: file_names.len(),
                dirs: dir_names.len(),
                total_files: file_names.len(),
                lines: Lines(0),
                tokens: Tokens(0),
            },
            file_names,
            dir_names,
        })
    }

    fn scan_dir_entries(&self, parent_real: &Path) -> Option<(Vec<String>, BTreeSet<String>)> {
        let mut file_names = Vec::new();
        let mut dir_names = BTreeSet::new();
        for entry in std::fs::read_dir(parent_real).ok()?.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if !self.check.walk.hidden && name.starts_with('.') {
                continue;
            }
            let Ok(ft) = entry.file_type() else {
                continue;
            };
            if ft.is_dir() {
                dir_names.insert(name);
            } else if ft.is_file()
                && self
                    .check
                    .walk
                    .accepts_file(self.root, self.config, &entry.path())
                    .unwrap_or(false)
            {
                file_names.push(name);
            }
        }
        file_names.sort();
        Some((file_names, dir_names))
    }

    fn passes_walk(&self) -> Result<bool> {
        if !self.check.walk.hidden
            && self
                .relative
                .components()
                .any(|c| c.as_os_str().to_string_lossy().starts_with('.'))
        {
            return Ok(false);
        }
        self.check
            .walk
            .accepts_file(self.root, self.config, self.real)
    }
}
