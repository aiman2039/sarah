use anyhow::Result;
use rayon::prelude::*;

use super::args::WalkArgs;
use crate::config::Config;
use crate::count::{Analysis, AnalyzeInput, FileStats};
use crate::walk::{FileEntry, Skipped, Walk, WalkFilters, WalkOptions};

/// Everything measured under one root, before limits turn it into a report.
pub struct Scan {
    pub config: Config,
    pub files: Vec<FileStats>,
    pub skipped: Skipped,
}

/// Include/exclude globs from the CLI plus `sarah.toml`.
struct Globs {
    include: Vec<String>,
    exclude: Vec<String>,
}

impl WalkArgs {
    /// Load config, walk, and measure every file in parallel.
    pub fn scan(&self, with_functions: bool) -> Result<Scan> {
        let config = Config::load(&self.path, self.config.as_deref())?;
        let globs = self.globs(&config);
        let Walk { files, mut skipped } = Walk::collect(&self.path, &self.walk_options(&globs))?;
        let files = self.analyze_in_parallel(files, with_functions, &mut skipped);
        Ok(Scan {
            config,
            files,
            skipped,
        })
    }

    fn globs(&self, config: &Config) -> Globs {
        Globs {
            include: [self.include.as_slice(), &config.include].concat(),
            exclude: [self.exclude.as_slice(), &config.exclude].concat(),
        }
    }

    fn walk_options<'a>(&self, globs: &'a Globs) -> WalkOptions<'a> {
        WalkOptions {
            filters: WalkFilters {
                include: &globs.include,
                exclude: &globs.exclude,
                default_excludes: !self.no_default_excludes,
            },
            hidden: self.hidden,
        }
    }

    /// Measure every file; generated ones are counted in `skipped` instead.
    fn analyze_in_parallel(
        &self,
        entries: Vec<FileEntry>,
        with_functions: bool,
        skipped: &mut Skipped,
    ) -> Vec<FileStats> {
        let analyses: Vec<Analysis> = entries
            .into_par_iter()
            .map(|entry| {
                FileStats::analyze(AnalyzeInput {
                    entry,
                    tokenizer: self.tokenizer,
                    with_functions,
                    skip_generated: !self.no_default_excludes,
                })
            })
            .collect();
        let mut files = Vec::with_capacity(analyses.len());
        for analysis in analyses {
            match analysis {
                Analysis::Counted(stats) => files.push(*stats),
                Analysis::Generated => skipped.generated = skipped.generated.saturating_add(1),
                Analysis::NotText => {}
            }
        }
        files
    }
}
