use std::path::PathBuf;

use clap::{Args as ClapArgs, Parser, Subcommand, ValueEnum};

use crate::count::Tokenizer;
use crate::report::SortBy;
use crate::units::{Cognitive, Cyclomatic, Depth, Lines, Params, Tokens};

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum Format {
    Table,
    Json,
    Md,
    /// GitHub Actions `::warning` annotations
    Github,
    /// SARIF 2.1.0 for code scanning
    Sarif,
    /// Self-contained HTML treemap of files by size
    Html,
}

/// Count tokens, lines and function sizes per file to help AI agents decide what to refactor.
#[derive(Parser, Debug)]
#[command(version, args_conflicts_with_subcommands = true)]
pub struct Args {
    #[command(subcommand)]
    pub command: Option<Command>,

    #[command(flatten)]
    pub check: CheckArgs,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Write sarah.toml with limits at today's maximums, so the code can only get better
    Init(WalkArgs),
    /// Lower the limits in sarah.toml to today's maximums (never raises them)
    Ratchet(RatchetArgs),
    /// Serve sarah to AI agents over the Model Context Protocol (stdio)
    Mcp,
}

/// What to walk and how to measure it; shared by every command.
#[derive(ClapArgs, Debug, Clone)]
pub struct WalkArgs {
    /// Directory to analyze
    #[arg(default_value = ".")]
    pub path: PathBuf,

    /// Token encoding (approximation; Claude's tokenizer is not public)
    #[arg(short, long, value_enum, default_value_t = Tokenizer::O200k)]
    pub tokenizer: Tokenizer,

    /// Config file [default: <PATH>/sarah.toml when it exists]
    #[arg(long, value_name = "FILE")]
    pub config: Option<PathBuf>,

    /// Only include paths matching these globs (repeatable)
    #[arg(short, long)]
    pub include: Vec<String>,

    /// Exclude paths matching these globs (repeatable)
    #[arg(short, long)]
    pub exclude: Vec<String>,

    /// Include hidden files and directories
    #[arg(long)]
    pub hidden: bool,

    /// Don't skip lockfiles, minified/source-map files and generated files (`@generated`, `DO NOT EDIT`)
    #[arg(long)]
    pub no_default_excludes: bool,
}

#[derive(ClapArgs, Debug)]
pub struct RatchetArgs {
    #[command(flatten)]
    pub walk: WalkArgs,

    /// Print the changes without writing sarah.toml
    #[arg(long)]
    pub dry_run: bool,
}

/// The default command: measure, warn, report.
#[derive(ClapArgs, Debug)]
pub struct CheckArgs {
    #[command(flatten)]
    pub walk: WalkArgs,

    #[arg(short, long, value_enum, default_value_t = Format::Table)]
    pub format: Format,

    #[arg(short, long, value_enum, default_value_t = SortBy::Tokens)]
    pub sort: SortBy,

    /// Rows to show in file/function tables (JSON always includes everything)
    #[arg(short = 'n', long, default_value_t = 20)]
    pub top: usize,

    /// [default: 8000]
    #[arg(long)]
    pub max_file_tokens: Option<Tokens>,

    /// [default: 500]
    #[arg(long)]
    pub max_file_lines: Option<Lines>,

    /// [default: 80]
    #[arg(long)]
    pub max_fn_lines: Option<Lines>,

    /// Warn on functions over this many tokens [default: off]
    #[arg(long)]
    pub max_fn_tokens: Option<Tokens>,

    /// Warn on functions nested deeper than this (if/loop/match levels) [default: off]
    #[arg(long)]
    pub max_fn_depth: Option<Depth>,

    /// Warn on functions with more parameters than this (`self` excluded) [default: off]
    #[arg(long)]
    pub max_fn_params: Option<Params>,

    /// Warn on functions over this cyclomatic complexity [default: off]
    #[arg(long)]
    pub max_fn_cyclomatic: Option<Cyclomatic>,

    /// Warn on functions over this cognitive complexity [default: off]
    #[arg(long)]
    pub max_fn_cognitive: Option<Cognitive>,

    /// Show per-folder totals in table/md output (JSON always has them)
    #[arg(long)]
    pub dirs: bool,

    /// Hide warnings already in this JSON report, unless they got worse
    #[arg(long, value_name = "FILE")]
    pub baseline: Option<PathBuf>,

    /// Only report warnings for files changed since this git revision (and their folders)
    #[arg(long, value_name = "REV")]
    pub changed_since: Option<String>,

    /// Ignore `sarah-allow` comments and report every warning
    #[arg(long)]
    pub no_allow: bool,

    /// Skip tree-sitter function extraction
    #[arg(long)]
    pub no_functions: bool,

    /// Exit with code 1 if any warnings are reported (errors always exit with 2)
    #[arg(long)]
    pub fail_on_warn: bool,

    /// Also write the full JSON report to this file, whatever --format is
    #[arg(long, value_name = "PATH")]
    pub json_out: Option<PathBuf>,
}
