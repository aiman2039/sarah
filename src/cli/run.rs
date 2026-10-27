use std::process::ExitCode;

use anyhow::Result;
use rayon::prelude::*;

use super::args::{Args, Format};
use super::output::CliOutput;
use crate::count::{AnalyzeInput, FileStats};
use crate::report::{FileLimits, Report, ReportInput, Thresholds};
use crate::walk::{FileEntry, Skipped, Walk, WalkFilters, WalkOptions};

impl Args {
    /// The whole program: walk, measure, aggregate, emit, report status.
    pub fn run(&self) -> Result<ExitCode> {
        let walk = Walk::collect(&self.path, &self.walk_options())?;
        let files = self.analyze_in_parallel(walk.files);
        let report = Report::build(self.report_input(files, walk.skipped));
        self.emit(&report)?;
        Ok(self.exit_code(&report))
    }

    fn walk_options(&self) -> WalkOptions<'_> {
        WalkOptions {
            filters: WalkFilters {
                include: &self.include,
                exclude: &self.exclude,
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

    fn report_input(&self, files: Vec<FileStats>, skipped: Skipped) -> ReportInput {
        ReportInput {
            files,
            skipped,
            sort: self.sort,
            limits: Thresholds {
                file: FileLimits {
                    tokens: self.max_file_tokens,
                    lines: self.max_file_lines,
                },
                fn_lines: self.max_fn_lines,
            },
            tokenizer: format!("{:?}", self.tokenizer).to_lowercase(),
        }
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
        Ok(match self.format {
            Format::Table => report.to_table(self.top),
            Format::Json => report.to_json()?,
            Format::Md => report.to_markdown(self.top),
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
