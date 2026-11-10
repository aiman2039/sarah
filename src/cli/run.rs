use std::process::ExitCode;
use std::time::Duration;

use anyhow::Result;

use super::args::{CheckArgs, Format};
use super::output::CliOutput;
use super::scan::{Scan, ScanOptions};
use crate::cache::ScanCache;
use crate::config::{Config, LimitPatch, LimitRules};
use crate::count::FileStats;
use crate::git::GitHelpers;
use crate::report::{Baseline, Report, ReportInput, View};
use crate::structure::{FolderPatch, FolderRules};
use crate::walk::Skipped;

impl CheckArgs {
    /// Build the report (or watch continuously), emit it, report status.
    pub fn run(&self) -> Result<ExitCode> {
        if self.hygiene.watch {
            return self.run_watch();
        }
        let report = self.report()?;
        self.emit(&report)?;
        Ok(self.exit_code(&report))
    }

    fn run_watch(&self) -> Result<ExitCode> {
        let cache = ScanCache::new();
        let mut last = Vec::new();
        loop {
            let current = self.walk.watch_fingerprint()?;
            if current != last {
                let report = self.report_with_cache(Some(&cache))?;
                self.emit(&report)?;
                last = current;
            }
            std::thread::sleep(Duration::from_millis(500));
        }
    }

    /// Walk, measure, aggregate and filter; nothing is printed.
    pub fn report(&self) -> Result<Report> {
        self.report_with_cache(None)
    }

    pub fn report_with_cache(&self, cache: Option<&ScanCache>) -> Result<Report> {
        let Scan {
            config,
            files,
            skipped,
        } = self.walk.scan_cached(ScanOptions {
            with_functions: !self.no_functions,
            cli_exclude_tests: self.hygiene.exclude_inline_tests,
            cache,
        })?;
        let limits = self.limit_rules(&config)?;
        let mut report = Report::build(self.report_input(files, skipped, &config)?);
        self.filter(&mut report, &limits)?;
        if let Some(rev) = &self.diff {
            report.diff = Some(self.compute_diff(rev, &config, &report)?);
        }
        Ok(report)
    }

    pub fn limit_rules(&self, config: &Config) -> Result<LimitRules> {
        LimitRules::with_base(&config.files, config.base_flags(), self.limit_flags())
    }

    pub fn folder_rules(&self, config: &Config) -> Result<FolderRules> {
        FolderRules::with_cli(config.preset, config.folder.clone(), self.folder_flags())
    }

    pub(super) fn report_input(
        &self,
        files: Vec<FileStats>,
        skipped: Skipped,
        config: &Config,
    ) -> Result<ReportInput> {
        let no_cycles = self.hygiene.no_cycles || config.no_cycles.unwrap_or(false);
        let max_duplicate_lines = self
            .hygiene
            .max_duplicate_lines
            .or(config.max_duplicate_lines);
        Ok(ReportInput {
            files,
            skipped,
            sort: self.sort,
            limits: self.limit_rules(config)?,
            folders: self.folder_rules(config)?,
            tokenizer: self.walk.tokenizer.to_string(),
            no_cycles,
            max_duplicate_lines,
        })
    }

    fn folder_flags(&self) -> FolderPatch {
        FolderPatch {
            files: self.folder.files,
            total_files: self.folder.total_files,
            dirs: self.folder.subdirs,
            depth: self.folder.depth,
        }
    }

    fn limit_flags(&self) -> LimitPatch {
        LimitPatch {
            max_file_tokens: self.max_file_tokens,
            max_file_lines: self.max_file_lines,
            max_file_fns: self.max_file_fns,
            max_file_imports: self.max_file_imports,
            max_fn_lines: self.fn_limits.lines,
            max_fn_tokens: self.fn_limits.tokens,
            max_fn_depth: self.fn_limits.depth,
            max_fn_params: self.fn_limits.params,
            max_fn_bool_params: self.fn_limits.bool_params,
            max_fn_cyclomatic: self.fn_limits.cyclomatic,
            max_fn_cognitive: self.fn_limits.cognitive,
            max_type_fields: self.max_type_fields,
            max_type_methods: self.max_type_methods,
            max_todos: self.hygiene.max_todos,
            unused_allow: self.hygiene.unused_allow.then_some(true),
            require_allow_reason: self.hygiene.require_allow_reason.then_some(true),
            exclude_inline_tests: self.hygiene.exclude_inline_tests.then_some(true),
        }
    }

    fn filter(&self, report: &mut Report, limits: &LimitRules) -> Result<()> {
        if !self.no_allow {
            report.apply_allows(limits);
        }
        if let Some(path) = &self.baseline {
            report.apply_baseline(&Baseline::load(path)?);
        }
        if let Some(rev) = &self.changed_since {
            report.retain_changed(&GitHelpers::changed_since(&self.walk.path, rev)?);
        }
        Ok(())
    }

    fn emit(&self, report: &Report) -> Result<()> {
        CliOutput::print_report(&self.render(report)?);
        if let Some(path) = &self.json_out {
            std::fs::write(path, report.to_json()?)?;
        }
        Ok(())
    }

    pub fn render(&self, report: &Report) -> Result<String> {
        let view = View {
            top: self.top,
            dirs: self.dirs,
        };
        Ok(match self.format {
            Format::Table => report.to_table(view),
            Format::Json => report.to_json()?,
            Format::Md => report.to_markdown(view),
            Format::Github => report.to_github(&self.walk.path),
            Format::Sarif => report.to_sarif(&self.walk.path)?,
            Format::Html => report.to_html()?,
        })
    }

    fn exit_code(&self, report: &Report) -> ExitCode {
        if self.fail_on_warn && !report.warnings.is_empty() {
            ExitCode::FAILURE
        } else {
            ExitCode::SUCCESS
        }
    }
}
