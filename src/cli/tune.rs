use std::fmt::Write as _;
use std::process::ExitCode;

use anyhow::{Context, Result, bail};

use super::args::{RatchetArgs, WalkArgs};
use super::output::CliOutput;
use crate::config::{Config, LimitPatch, LimitRules};
use crate::ratchet::{Maxima, Tightened};

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
        let no_rules = LimitRules::new(&[], LimitPatch::default())?;
        let text = Maxima::of(&scan.files, &no_rules).init_toml();
        std::fs::write(&target, &text)
            .with_context(|| format!("cannot write {}", target.display()))?;
        CliOutput::print_report(&format!("Wrote {}:\n\n{text}", target.display()));
        Ok(ExitCode::SUCCESS)
    }
}

impl RatchetArgs {
    /// `sarah ratchet`: lower the `[[files]]` limits to today's maximums.
    pub fn run(&self) -> Result<ExitCode> {
        let Self { walk, dry_run } = self;
        let target = Config::target(&walk.path, walk.config.as_deref());
        let text = std::fs::read_to_string(&target)
            .with_context(|| format!("cannot read {}; run `sarah init` first", target.display()))?;
        let scan = walk.scan(true)?;
        let rules = LimitRules::new(&scan.config.files, LimitPatch::default())?;
        let tightened = Maxima::of(&scan.files, &rules).tighten(&text)?;
        if !dry_run && !tightened.changes.is_empty() {
            std::fs::write(&target, &tightened.text)
                .with_context(|| format!("cannot write {}", target.display()))?;
        }
        CliOutput::print_report(&Self::summary(&tightened, *dry_run));
        Ok(ExitCode::SUCCESS)
    }

    fn summary(tightened: &Tightened, dry_run: bool) -> String {
        if tightened.changes.is_empty() {
            return "No limit can be lowered.".to_string();
        }
        let verb = if dry_run { "Would lower" } else { "Lowered" };
        let mut out = format!("{verb} {} limits:\n", tightened.changes.len());
        for c in &tightened.changes {
            let _ = writeln!(
                out,
                "- [[files]] {:?}: {} {} -> {}",
                c.rule,
                c.key.name(),
                c.from,
                c.to
            );
        }
        out
    }
}
