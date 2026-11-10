use std::collections::HashSet;
use std::path::{Path, PathBuf};

use anyhow::Result;

use super::args::CheckArgs;
use crate::config::Config;
use crate::count::{Analysis, FileStats, SourceInput};
use crate::git::GitHelpers;
use crate::report::{Baseline, DiffSummary, Report};
use crate::walk::Skipped;

struct BaseCtx<'a> {
    base: &'a str,
    changed: &'a HashSet<PathBuf>,
    config: &'a Config,
}

impl CheckArgs {
    /// Reconstruct the report at the merge base with `rev` by re-measuring only changed paths.
    pub(super) fn compute_diff(
        &self,
        rev: &str,
        config: &Config,
        current: &Report,
    ) -> Result<DiffSummary> {
        let base_hash = GitHelpers::merge_base(&self.walk.path, rev)?;
        let changed = GitHelpers::changed_from_base(&self.walk.path, &base_hash)?;
        let ctx = BaseCtx {
            base: &base_hash,
            changed: &changed,
            config,
        };
        let base_files = self.files_at_base(ctx, &current.files)?;
        let limits = self.limit_rules(config)?;
        let mut base_report =
            Report::build(self.report_input(base_files, Skipped::default(), config)?);
        if !self.no_allow {
            base_report.apply_allows(&limits);
        }
        if let Some(path) = &self.baseline {
            base_report.apply_baseline(&Baseline::load(path)?);
        }
        if let Some(since) = &self.changed_since {
            base_report.retain_changed(&GitHelpers::changed_since(&self.walk.path, since)?);
        }
        Ok(DiffSummary::between(rev.to_string(), &base_report, current))
    }

    fn files_at_base(
        &self,
        ctx: BaseCtx<'_>,
        current_files: &[FileStats],
    ) -> Result<Vec<FileStats>> {
        let BaseCtx {
            base,
            changed,
            config,
        } = ctx;
        let mut out: Vec<FileStats> = current_files
            .iter()
            .filter(|f| !changed.contains(&f.path))
            .cloned()
            .collect();
        let root = dunce::canonicalize(&self.walk.path)?;
        for rel in changed {
            if !self.walk.hidden && Self::is_hidden_rel(rel) {
                continue;
            }
            if !self.walk.accepts_file(&root, config, &root.join(rel))? {
                continue;
            }
            let Some(text) = GitHelpers::file_at_base(&self.walk.path, base, rel) else {
                continue;
            };
            if let Analysis::Counted(stats) = FileStats::analyze_source(SourceInput {
                path: rel.clone(),
                text: &text,
                tokenizer: self.walk.tokenizer,
                with_functions: !self.no_functions,
                skip_generated: !self.walk.no_default_excludes,
            }) {
                out.push(*stats);
            }
        }
        Ok(out)
    }

    fn is_hidden_rel(rel: &Path) -> bool {
        rel.components()
            .any(|c| c.as_os_str().to_string_lossy().starts_with('.'))
    }
}
