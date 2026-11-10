use std::fmt::Write as _;
use std::process::ExitCode;

use anyhow::{Context, Result, bail};

use super::args::{RatchetArgs, WalkArgs};
use super::output::CliOutput;
use crate::config::{Config, LimitPatch, LimitRules};
use crate::ratchet::{Maxima, Tightened};
use crate::structure::FolderRules;

impl WalkArgs {
    /// `sarah init`: write `sarah.toml` with limits at today's maximums.
    pub fn init(&self) -> Result<ExitCode> {
        let target = Config::target(&self.path, self.config.as_deref());
        if target.exists() {
            bail!(
                "{} already exists; use `sarah ratchet` to tighten it",
                target.display()
            );
        }
        let scan = self.scan(true)?;
        let no_files = LimitRules::new(&[], LimitPatch::default())?;
        let no_folders = FolderRules::new(None, Vec::new())?;
        let text = Maxima::of(&scan.files, &no_files, &no_folders).init_toml();
        std::fs::write(&target, &text)
            .with_context(|| format!("cannot write {}", target.display()))?;
        CliOutput::print_report(&format!("Wrote {}:\n\n{text}", target.display()));
        Ok(ExitCode::SUCCESS)
    }
}

impl RatchetArgs {
    /// `sarah ratchet`: lower the `[[files]]` and `[[folder]]` limits to today's maximums.
    pub fn run(&self) -> Result<ExitCode> {
        let Self {
            walk,
            dry_run,
            check,
        } = self;
        let write = !dry_run && !check;
        let target = Config::target(&walk.path, walk.config.as_deref());
        let text = std::fs::read_to_string(&target)
            .with_context(|| format!("cannot read {}; run `sarah init` first", target.display()))?;
        let scan = walk.scan(true)?;
        let file_rules = LimitRules::new(&scan.config.files, LimitPatch::default())?;
        let folder_rules = FolderRules::new(scan.config.preset, scan.config.folder)?;
        let tightened = Maxima::of(&scan.files, &file_rules, &folder_rules).tighten(&text)?;
        if write && !tightened.changes.is_empty() {
            std::fs::write(&target, &tightened.text)
                .with_context(|| format!("cannot write {}", target.display()))?;
        }
        CliOutput::print_report(&Self::summary(&tightened, !write));
        if *check && !tightened.changes.is_empty() {
            Ok(ExitCode::FAILURE)
        } else {
            Ok(ExitCode::SUCCESS)
        }
    }

    fn summary(tightened: &Tightened, preview: bool) -> String {
        if tightened.changes.is_empty() {
            return "No limit can be lowered.".to_string();
        }
        let verb = if preview { "Would lower" } else { "Lowered" };
        let mut out = format!("{verb} {} limits:\n", tightened.changes.len());
        for c in &tightened.changes {
            let _ = writeln!(
                out,
                "- [[{}]] {:?}: {} {} -> {}",
                c.section, c.rule, c.key, c.from, c.to
            );
        }
        out
    }
}
