mod allow;
mod cli;
mod config;
mod count;
mod functions;
mod git;
mod mcp;
mod ratchet;
mod report;
mod structure;
mod units;
mod walk;

use std::process::ExitCode;

use clap::Parser;

fn main() -> ExitCode {
    cli::Args::parse().run()
}
