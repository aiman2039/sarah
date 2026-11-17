use std::path::{Path, PathBuf};

use anyhow::Result;
use rayon::prelude::*;

use super::args::WalkArgs;
use crate::cache::ScanCache;
use crate::config::{Config, LimitPatch, LimitRules};
use crate::count::{Analysis, AnalysisRun, AnalyzeInput, FileStats};
use crate::structure::FolderRules;
use crate::walk::{FileEntry, Skipped, Walk, WalkFilters, WalkOptions};

/// `(realpath, (mtime_nanos, byte_len))` for `--watch` change detection.
pub type FileSig = (PathBuf, Option<(u128, u64)>);

/// Everything measured under one root, before limits turn it into a report.
pub struct Scan {
    pub config: Config,
    pub files: Vec<FileStats>,
    pub skipped: Skipped,
}

#[derive(Clone, Copy)]
pub struct ScanOptions<'a> {
    pub with_functions: bool,
    /// CLI limits and switches, applied after `sarah.toml` (as in the report).
    pub cli: LimitPatch,
    pub cache: Option<&'a ScanCache>,
}

/// What decides the analysis of each walked path.
struct ScanRules {
    limits: LimitRules,
    /// A `[[folder]]` rule sets a test ratio, so every file's inline tests are measured.
    folder_test_ratio: bool,
}

/// Include/exclude globs from the CLI plus `sarah.toml`.
struct Globs {
    include: Vec<String>,
    exclude: Vec<String>,
}

impl WalkArgs {
    /// Load config, walk, and measure every file in parallel.
    pub fn scan(&self, with_functions: bool) -> Result<Scan> {
        self.scan_cached(&ScanOptions {
            with_functions,
            cli: LimitPatch::default(),
            cache: None,
        })
    }

    pub fn scan_cached(&self, opts: &ScanOptions<'_>) -> Result<Scan> {
        let config = Config::load(&self.path, self.config.as_deref())?;
        let rules = ScanRules {
            limits: LimitRules::with_base(&config.files, config.base_flags(), opts.cli)?,
            folder_test_ratio: FolderRules::new(config.preset, config.folder.clone())?
                .has_test_ratio(),
        };
        let globs = self.globs(&config);
        let Walk { files, mut skipped } = Walk::collect(&self.path, &self.walk_options(&globs))?;
        let analyses = self.analyze_in_parallel(files, opts, &rules);
        let files = Self::keep_counted(analyses, &mut skipped);
        Ok(Scan {
            config,
            files,
            skipped,
        })
    }

    /// Lightweight `(realpath, (mtime_nanos, len))` fingerprint of walked files and `sarah.toml`.
    pub fn watch_fingerprint(&self) -> Result<Vec<FileSig>> {
        let config = Config::load(&self.path, self.config.as_deref())?;
        let globs = self.globs(&config);
        let Walk { files, .. } = Walk::collect(&self.path, &self.walk_options(&globs))?;
        let cfg_path = Config::target(&self.path, self.config.as_deref());
        let mut sigs: Vec<_> = files
            .into_iter()
            .map(|e| {
                let sig = ScanCache::file_sig(&e.real);
                (e.real, sig)
            })
            .collect();
        sigs.push((cfg_path.clone(), ScanCache::file_sig(&cfg_path)));
        Ok(sigs)
    }

    /// Whether `path` under `root` passes the CLI and `config` filters.
    pub fn accepts_file(&self, root: &Path, config: &Config, path: &Path) -> Result<bool> {
        let globs = self.globs(config);
        self.walk_options(&globs).filters.accepts(root, path)
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

    fn analyze_in_parallel(
        &self,
        entries: Vec<FileEntry>,
        opts: &ScanOptions<'_>,
        rules: &ScanRules,
    ) -> Vec<Analysis> {
        let run = AnalysisRun {
            tokenizer: self.tokenizer,
            with_functions: opts.with_functions,
            skip_generated: !self.no_default_excludes,
        };
        entries
            .into_par_iter()
            .map(|entry| {
                let input = AnalyzeInput {
                    run,
                    rules: (rules.limits).analysis_for(&entry.display, rules.folder_test_ratio),
                    entry,
                };
                match opts.cache {
                    Some(c) => c.analyze(input),
                    None => FileStats::analyze(input),
                }
            })
            .collect()
    }

    /// Counted files; generated ones are added to `skipped`.
    fn keep_counted(analyses: Vec<Analysis>, skipped: &mut Skipped) -> Vec<FileStats> {
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
