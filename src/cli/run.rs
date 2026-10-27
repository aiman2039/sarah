use std::process::ExitCode;

use anyhow::Result;
use rayon::prelude::*;

use super::args::{Args, Format};
use super::output::CliOutput;
use crate::config::{Config, LimitPatch, LimitRules};
use crate::count::{AnalyzeInput, FileStats};
use crate::git::GitHelpers;
use crate::report::{Baseline, Report, ReportInput, View};
use crate::structure::FolderRules;
use crate::walk::{FileEntry, Skipped, Walk, WalkFilters, WalkOptions};

/// Exit code for errors; 1 means "warnings with --fail-on-warn".
const EXIT_ERROR: u8 = 2;

/// Include/exclude globs from the CLI plus `sarah.toml`.
struct Globs {
    include: Vec<String>,
    exclude: Vec<String>,
}

impl Args {
    /// The whole program; errors are printed and exit with code 2.
    pub fn run(&self) -> ExitCode {
        self.try_run().unwrap_or_else(|err| {
            CliOutput::print_error(&err);
            ExitCode::from(EXIT_ERROR)
        })
    }

    /// Load config, walk, measure, aggregate, filter, emit, report status.
    fn try_run(&self) -> Result<ExitCode> {
        let config = Config::load(&self.path, self.config.as_deref())?;
        let globs = self.globs(&config);
        let walk = Walk::collect(&self.path, &self.walk_options(&globs))?;
        let files = self.analyze_in_parallel(walk.files);
        let mut report = Report::build(self.report_input(files, walk.skipped, config)?);
        self.filter(&mut report)?;
        self.emit(&report)?;
        Ok(self.exit_code(&report))
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

    fn analyze_in_parallel(&self, entries: Vec<FileEntry>) -> Vec<FileStats> {
        entries
            .into_par_iter()
            .filter_map(|entry| {
                FileStats::analyze(AnalyzeInput {
                    entry,
                    tokenizer: self.tokenizer,
                    with_functions: !self.no_functions,
                })
            })
            .collect()
    }

    fn report_input(
        &self,
        files: Vec<FileStats>,
        skipped: Skipped,
        config: Config,
    ) -> Result<ReportInput> {
        Ok(ReportInput {
            files,
            skipped,
            sort: self.sort,
            limits: LimitRules::new(&config.files, self.limit_flags())?,
            folders: FolderRules::new(config.preset, config.folder)?,
            tokenizer: self.tokenizer.to_string(),
        })
    }

    fn limit_flags(&self) -> LimitPatch {
        LimitPatch {
            max_file_tokens: self.max_file_tokens,
            max_file_lines: self.max_file_lines,
            max_fn_lines: self.max_fn_lines,
            max_fn_tokens: self.max_fn_tokens,
            max_fn_depth: self.max_fn_depth,
        }
    }

    /// Apply `--baseline`, then `--changed-since`.
    fn filter(&self, report: &mut Report) -> Result<()> {
        if let Some(path) = &self.baseline {
            report.apply_baseline(&Baseline::load(path)?);
        }
        if let Some(rev) = &self.changed_since {
            report.retain_changed(&GitHelpers::changed_since(&self.path, rev)?);
        }
        Ok(())
    }

    /// Print the report in the chosen format, then write `--json-out` if set.
    fn emit(&self, report: &Report) -> Result<()> {
        CliOutput::print_report(&self.render(report)?);
        if let Some(path) = &self.json_out {
            std::fs::write(path, report.to_json()?)?;
        }
        Ok(())
    }

    fn render(&self, report: &Report) -> Result<String> {
        let view = View {
            top: self.top,
            dirs: self.dirs,
        };
        Ok(match self.format {
            Format::Table => report.to_table(view),
            Format::Json => report.to_json()?,
            Format::Md => report.to_markdown(view),
            Format::Github => report.to_github(&self.path),
            Format::Sarif => report.to_sarif(&self.path)?,
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
