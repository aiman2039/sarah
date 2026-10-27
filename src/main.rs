mod cli;
mod count;
mod functions;
mod report;
mod units;
mod walk;

use std::process::ExitCode;

use anyhow::Result;
use clap::Parser;

fn main() -> Result<ExitCode> {
    cli::Args::parse().run()
}
