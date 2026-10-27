use std::path::PathBuf;

use clap::{Parser, ValueEnum};

use crate::count::Tokenizer;
use crate::report::SortBy;
use crate::units::{Lines, Tokens};

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum Format {
    Table,
    Json,
    Md,
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

    #[arg(long, default_value_t = Tokens(8000))]
    pub max_file_tokens: Tokens,

    #[arg(long, default_value_t = Lines(500))]
    pub max_file_lines: Lines,

    #[arg(long, default_value_t = Lines(80))]
    pub max_fn_lines: Lines,

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

    /// Exit with code 1 if any warnings are reported
    #[arg(long)]
    pub fail_on_warn: bool,

    /// Also write the full JSON report to this file, whatever --format is
    #[arg(long, value_name = "PATH")]
    pub json_out: Option<PathBuf>,
}
