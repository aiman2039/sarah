use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use clap::Parser;
use serde_json::json;

use super::requests::{CheckFileRequest, ReportFormat, ReportRequest, WarningsRequest};
use crate::cli::{Args, CheckArgs};
use crate::report::Report;

/// Stateless tool bodies: each parses CLI-style arguments and returns the text to send back.
pub(super) struct ToolHelpers;

impl ToolHelpers {
    pub(super) fn report(request: ReportRequest) -> Result<String> {
        let ReportRequest { path, format, args } = request;
        let format = match format {
            ReportFormat::Md => "md",
            ReportFormat::Json => "json",
        };
        let flags = [vec!["-f".to_string(), format.to_string()], args].concat();
        let check = Self::check_args(Self::root(path.as_deref()), &flags)?;
        check.render(&check.report()?)
    }

    pub(super) fn warnings(request: WarningsRequest) -> Result<String> {
        let WarningsRequest {
            path,
            changed_since,
            args,
        } = request;
        let since = changed_since
            .map(|rev| vec!["--changed-since".to_string(), rev])
            .unwrap_or_default();
        let check = Self::check_args(Self::root(path.as_deref()), &[since, args].concat())?;
        let report = check.report()?;
        Ok(serde_json::to_string_pretty(&json!({
            "warnings": report.warnings,
            "allowed": report.allowed,
            "baselined": report.baselined,
        }))?)
    }

    /// One file's sizes, functions and warnings, under the root's `sarah.toml`.
    pub(super) fn check_file(request: CheckFileRequest) -> Result<String> {
        let CheckFileRequest { file, root, args } = request;
        let root = Self::root(root.as_deref());
        let relative = Self::relative_to(&root, Path::new(&file))?;
        let glob = relative.to_string_lossy().replace('\\', "/");
        let flags = [vec!["--include".to_string(), glob], args].concat();
        let check = Self::check_args(root, &flags)?;
        let report = check.report()?;
        Self::file_json(&report, &relative)
    }

    fn file_json(report: &Report, relative: &Path) -> Result<String> {
        let Some(stats) = report.files.iter().find(|f| f.path == relative) else {
            bail!(
                "{} was not analyzed (excluded, ignored, generated or not text)",
                relative.display()
            );
        };
        // Folder rules see only this one file here, so their warnings would be wrong.
        let warnings: Vec<_> = report
            .warnings
            .iter()
            .filter(|w| w.path == relative && !w.kind.is_folder())
            .collect();
        Ok(serde_json::to_string_pretty(&json!({
            "file": stats,
            "warnings": warnings,
        }))?)
    }

    fn root(path: Option<&str>) -> PathBuf {
        PathBuf::from(path.unwrap_or("."))
    }

    fn relative_to(root: &Path, file: &Path) -> Result<PathBuf> {
        let root = dunce::canonicalize(root)
            .with_context(|| format!("cannot resolve {}", root.display()))?;
        let joined = if file.is_absolute() {
            file.to_path_buf()
        } else {
            root.join(file)
        };
        let real = dunce::canonicalize(&joined)
            .with_context(|| format!("cannot resolve {}", joined.display()))?;
        let relative = real
            .strip_prefix(&root)
            .with_context(|| format!("{} is outside {}", real.display(), root.display()))?;
        Ok(relative.to_path_buf())
    }

    /// Parse `args` exactly like the command line, so every flag and `sarah.toml` work.
    fn check_args(root: PathBuf, args: &[String]) -> Result<CheckArgs> {
        let argv = ["sarah".into(), root.into_os_string()]
            .into_iter()
            .chain(args.iter().map(Into::into));
        let parsed = Args::try_parse_from(argv).context("invalid sarah arguments")?;
        if parsed.command.is_some() {
            bail!("subcommands are not available as MCP arguments");
        }
        Ok(parsed.check)
    }
}
