mod allow;
mod cache;
mod cli;
mod config;
mod count;
mod duplicate;
mod functions;
mod git;
mod graph;
mod lang;
mod lines;
mod lsp;
mod mcp;
mod ratchet;
mod render;
mod report;
mod structure;
mod units;
mod visibility;
mod walk;

use std::process::ExitCode;

use clap::Parser;

fn main() -> ExitCode {
    cli::Args::parse().run()
}
