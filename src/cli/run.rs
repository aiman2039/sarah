use std::process::ExitCode;

use anyhow::Result;

use super::args::{CheckArgs, Format};
use super::output::CliOutput;
use super::scan::Scan;
use crate::config::{Config, LimitPatch, LimitRules};
use crate::count::FileStats;
use crate::git::GitHelpers;
use crate::report::{Baseline, Report, ReportInput, View};
use crate::structure::FolderRules;
use crate::walk::Skipped;

impl CheckArgs {
    /// Build the report, emit it, report status.
    pub fn run(&self) -> Result<ExitCode> {
        let report = self.report()?;
        self.emit(&report)?;
        Ok(self.exit_code(&report))
    }

    /// Walk, measure, aggregate and filter; nothing is printed.
    pub fn report(&self) -> Result<Report> {
        let Scan {
            config,
            files,
            skipped,
        } = self.walk.scan(!self.no_functions)?;
        let mut report = Report::build(self.report_input(files, skipped, config)?);
        self.filter(&mut report)?;
        Ok(report)
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
            tokenizer: self.walk.tokenizer.to_string(),
        })
    }

    fn limit_flags(&self) -> LimitPatch {
        LimitPatch {
            max_file_tokens: self.max_file_tokens,
            max_file_lines: self.max_file_lines,
            max_fn_lines: self.max_fn_lines,
            max_fn_tokens: self.max_fn_tokens,
            max_fn_depth: self.max_fn_depth,
            max_fn_params: self.max_fn_params,
            max_fn_cyclomatic: self.max_fn_cyclomatic,
            max_fn_cognitive: self.max_fn_cognitive,
        }
    }

    /// Apply `sarah-allow` comments (unless `--no-allow`), `--baseline`, then `--changed-since`.
    fn filter(&self, report: &mut Report) -> Result<()> {
        if !self.no_allow {
            report.apply_allows();
        }
        if let Some(path) = &self.baseline {
            report.apply_baseline(&Baseline::load(path)?);
        }
        if let Some(rev) = &self.changed_since {
            report.retain_changed(&GitHelpers::changed_since(&self.walk.path, rev)?);
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

    /// The report in the chosen `--format`.
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
