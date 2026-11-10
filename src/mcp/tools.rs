use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use clap::Parser;
use serde_json::json;

use super::requests::{CheckFileRequest, ReportFormat, ReportRequest, WarningsRequest};
use crate::cli::{Args, CheckArgs};
use crate::config::Config;
use crate::count::{Analysis, AnalyzeInput, FileStats};
use crate::report::{Report, ReportInput};
use crate::structure::{DirNode, DirStats, FolderRules};
use crate::units::{Depth, Lines, Tokens};
use crate::walk::{FileEntry, Skipped};

/// Stateless tool bodies: each parses CLI-style arguments and returns the text to send back.
pub(super) struct ToolHelpers;

struct SingleFileCheck<'a> {
    check: &'a CheckArgs,
    config: &'a Config,
    root: &'a Path,
    real: &'a Path,
    relative: &'a Path,
}

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

    /// Direct single-file check: reads only `file` and its parent folder (never walks the repo).
    pub(super) fn check_file(request: CheckFileRequest) -> Result<String> {
        let CheckFileRequest { file, root, args } = request;
        let root_arg = Self::root(root.as_deref());
        let check = Self::check_args(root_arg.clone(), &args)?;
        let (canonical_root, real, relative) = Self::resolve_file(&root_arg, Path::new(&file))?;
        let config = Config::load(&canonical_root, check.walk.config.as_deref())?;
        let ctx = SingleFileCheck {
            check: &check,
            config: &config,
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

impl SingleFileCheck<'_> {
    fn execute(&self) -> Result<String> {
        let unanalyzed = || {
            format!(
                "{} was not analyzed (excluded, ignored, generated or not text)",
                self.relative.display()
            )
        };
        if !self.passes_walk()? {
            bail!("{}", unanalyzed());
        }
        let Analysis::Counted(stats) = FileStats::analyze(AnalyzeInput {
            entry: FileEntry {
                real: self.real.to_path_buf(),
                display: self.relative.to_path_buf(),
            },
            tokenizer: self.check.walk.tokenizer,
            with_functions: !self.check.no_functions,
            skip_generated: !self.check.walk.no_default_excludes,
        }) else {
            bail!("{}", unanalyzed());
        };
        self.render_json(*stats)
    }

    fn render_json(&self, stats: FileStats) -> Result<String> {
        let limits = self.check.limit_rules(self.config)?;
        let folders = FolderRules::new(self.config.preset, self.config.folder.clone())?;
        let mut report = Report::build(ReportInput {
            files: vec![stats],
            skipped: Skipped::default(),
            sort: self.check.sort,
            limits: self.check.limit_rules(self.config)?,
            folders: FolderRules::new(self.config.preset, self.config.folder.clone())?,
            tokenizer: self.check.walk.tokenizer.to_string(),
        });
        report.warnings.retain(|w| !w.kind.is_folder());
        if let Some(parent_node) = self.read_parent_dir() {
            let parent_rel = &parent_node.stats.path;
            report.warnings.extend(
                folders
                    .check_single_dir(&parent_node)
                    .into_iter()
                    .filter(|w| w.path == self.relative || &w.path == parent_rel),
            );
        }
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
        let parent_rel = match self.relative.parent() {
            Some(p) if !p.as_os_str().is_empty() => p.to_path_buf(),
            _ => PathBuf::from("."),
        };
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
