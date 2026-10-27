use std::path::PathBuf;

use clap::{Parser, ValueEnum};

use crate::count::Tokenizer;
use crate::report::SortBy;
use crate::units::{Depth, Lines, Tokens};

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum Format {
    Table,
    Json,
    Md,
    /// GitHub Actions `::warning` annotations
    Github,
    /// SARIF 2.1.0 for code scanning
    Sarif,
}

/// Count tokens, lines and function sizes per file to help AI agents decide what to refactor.
#[derive(Parser, Debug)]
#[command(version)]
pub struct Args {
    /// Directory to analyze
    #[arg(default_value = ".")]
    pub path: PathBuf,

    #[arg(short, long, value_enum, default_value_t = Format::Table)]
    pub format: Format,

    #[arg(short, long, value_enum, default_value_t = SortBy::Tokens)]
    pub sort: SortBy,

    /// Rows to show in file/function tables (JSON always includes everything)
    #[arg(short = 'n', long, default_value_t = 20)]
    pub top: usize,

    /// Token encoding (approximation; Claude's tokenizer is not public)
    #[arg(short, long, value_enum, default_value_t = Tokenizer::O200k)]
    pub tokenizer: Tokenizer,

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

    /// Show per-folder totals in table/md output (JSON always has them)
    #[arg(long)]
    pub dirs: bool,

    /// Config file [default: <PATH>/sarah.toml when it exists]
    #[arg(long, value_name = "FILE")]
    pub config: Option<PathBuf>,

    /// Hide warnings already in this JSON report, unless they got worse
    #[arg(long, value_name = "FILE")]
    pub baseline: Option<PathBuf>,

    /// Only report warnings for files changed since this git revision (and their folders)
    #[arg(long, value_name = "REV")]
    pub changed_since: Option<String>,

    /// Skip tree-sitter function extraction
    #[arg(long)]
    pub no_functions: bool,

    /// Only include paths matching these globs (repeatable)
    #[arg(short, long)]
    pub include: Vec<String>,

    /// Exclude paths matching these globs (repeatable)
    #[arg(short, long)]
    pub exclude: Vec<String>,

    /// Include hidden files and directories
    #[arg(long)]
    pub hidden: bool,

    /// Don't skip lockfiles and minified/source-map files
    #[arg(long)]
    pub no_default_excludes: bool,

    /// Exit with code 1 if any warnings are reported (errors always exit with 2)
    #[arg(long)]
    pub fail_on_warn: bool,

    /// Also write the full JSON report to this file, whatever --format is
    #[arg(long, value_name = "PATH")]
    pub json_out: Option<PathBuf>,
}
