use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use clap::Parser;
use serde_json::json;

use super::requests::{
    CheckFileRequest, DiffRequest, RatchetToolRequest, ReportFormat, ReportRequest, WarningsRequest,
};
use super::single_file::SingleFileCheck;
use crate::cache::ScanCache;
use crate::cli::{Args, CheckArgs, ScanOptions};
use crate::config::{Config, LimitPatch, LimitRules};
use crate::ratchet::Maxima;
use crate::structure::FolderRules;

/// Stateless tool bodies: each parses CLI-style arguments and returns the text to send back.
pub(super) struct ToolHelpers;

impl ToolHelpers {
    pub(super) fn report(request: ReportRequest, cache: &ScanCache) -> Result<String> {
        let ReportRequest { path, format, args } = request;
        let format = match format {
            ReportFormat::Md => "md",
            ReportFormat::Json => "json",
        };
        let flags = [vec!["-f".to_string(), format.to_string()], args].concat();
        let check = Self::check_args(Self::root(path.as_deref()), &flags)?;
        check.render(&check.report_with_cache(Some(cache))?)
    }

    pub(super) fn warnings(request: WarningsRequest, cache: &ScanCache) -> Result<String> {
        let WarningsRequest {
            path,
            changed_since,
            args,
        } = request;
        let since = changed_since
            .map(|rev| vec!["--changed-since".to_string(), rev])
            .unwrap_or_default();
        let check = Self::check_args(Self::root(path.as_deref()), &[since, args].concat())?;
        let report = check.report_with_cache(Some(cache))?;
        Ok(serde_json::to_string_pretty(&json!({
            "warnings": report.warnings,
            "allowed": report.allowed,
            "baselined": report.baselined,
        }))?)
    }

    pub(super) fn diff(request: DiffRequest, cache: &ScanCache) -> Result<String> {
        let DiffRequest { path, rev, args } = request;
        let flags = [vec!["--diff".to_string(), rev], args].concat();
        let check = Self::check_args(Self::root(path.as_deref()), &flags)?;
        let report = check.report_with_cache(Some(cache))?;
        Ok(serde_json::to_string_pretty(&json!({
            "diff": report.diff,
            "warnings": report.warnings,
        }))?)
    }

    pub(super) fn ratchet(request: RatchetToolRequest, cache: &ScanCache) -> Result<String> {
        let RatchetToolRequest { path, dry_run } = request;
        let root = Self::root(path.as_deref());
        let check = Self::check_args(root.clone(), &[])?;
        let target = Config::target(&root, None);
        let text = std::fs::read_to_string(&target)
            .with_context(|| format!("cannot read {}; run `sarah init` first", target.display()))?;
        let scan = check.walk.scan_cached(&ScanOptions {
            with_functions: true,
            cli: LimitPatch::default(),
            cache: Some(cache),
        })?;
        let file_rules = LimitRules::new(&scan.config.files, LimitPatch::default())?;
        let folder_rules = FolderRules::new(scan.config.preset, scan.config.folder)?;
        let tightened = Maxima::of(&scan.files, &file_rules, &folder_rules).tighten(&text)?;
        if !dry_run && !tightened.changes.is_empty() {
            std::fs::write(&target, &tightened.text)?;
        }
        let changes: Vec<_> = tightened
            .changes
            .iter()
            .map(|c| json!({ "section": c.section, "rule": c.rule, "key": c.key, "from": c.from, "to": c.to }))
            .collect();
        Ok(serde_json::to_string_pretty(&json!({
            "can_lower": !changes.is_empty(),
            "dry_run": dry_run,
            "changes": changes,
        }))?)
    }

    /// Direct single-file check: reads only `file` and its parent folder (never walks the repo).
    pub(super) fn check_file(request: CheckFileRequest, cache: &ScanCache) -> Result<String> {
        let CheckFileRequest { file, root, args } = request;
        let root_arg = Self::root(root.as_deref());
        let check = Self::check_args(root_arg.clone(), &args)?;
        let (canonical_root, real, relative) = Self::resolve_file(&root_arg, Path::new(&file))?;
        let config = Config::load(&canonical_root, check.walk.config.as_deref())?;
        let ctx = SingleFileCheck {
            check: &check,
            config: &config,
            cache,
            root: &canonical_root,
            real: &real,
            relative: &relative,
        };
        ctx.execute()
    }

    fn root(path: Option<&str>) -> PathBuf {
        PathBuf::from(path.unwrap_or("."))
    }

    fn resolve_file(root: &Path, file: &Path) -> Result<(PathBuf, PathBuf, PathBuf)> {
        let canonical_root = dunce::canonicalize(root)
            .with_context(|| format!("cannot resolve {}", root.display()))?;
        let joined = if file.is_absolute() {
            file.to_path_buf()
        } else {
            canonical_root.join(file)
        };
        let real = dunce::canonicalize(&joined)
            .with_context(|| format!("cannot resolve {}", joined.display()))?;
        let relative = real
            .strip_prefix(&canonical_root)
            .with_context(|| format!("{} is outside {}", real.display(), canonical_root.display()))?
            .to_path_buf();
        Ok((canonical_root, real, relative))
    }

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
