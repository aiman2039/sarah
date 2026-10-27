mod count;
mod functions;
mod report;
mod walk;

use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::Result;
use clap::{Parser, ValueEnum};
use rayon::prelude::*;

use count::Tokenizer;
use report::{Report, SortBy, Thresholds};

#[derive(Clone, Copy, Debug, ValueEnum)]
enum Format {
    Table,
    Json,
    Md,
}

/// Count tokens, lines and function sizes per file to help AI agents decide what to refactor.
#[derive(Parser, Debug)]
#[command(version)]
struct Args {
    /// Directory to analyze
    #[arg(default_value = ".")]
    path: PathBuf,

    #[arg(short, long, value_enum, default_value_t = Format::Table)]
    format: Format,

    #[arg(short, long, value_enum, default_value_t = SortBy::Tokens)]
    sort: SortBy,

    /// Rows to show in file/function tables (JSON always includes everything)
    #[arg(short = 'n', long, default_value_t = 20)]
    top: usize,

    /// Token encoding (approximation; Claude's tokenizer is not public)
    #[arg(short, long, value_enum, default_value_t = Tokenizer::O200k)]
    tokenizer: Tokenizer,

    #[arg(long, default_value_t = 8000)]
    max_file_tokens: usize,

    #[arg(long, default_value_t = 500)]
    max_file_lines: usize,

    #[arg(long, default_value_t = 80)]
    max_fn_lines: usize,

    /// Skip tree-sitter function extraction
    #[arg(long)]
    no_functions: bool,

    /// Only include paths matching these globs (repeatable)
    #[arg(short, long)]
    include: Vec<String>,

    /// Exclude paths matching these globs (repeatable)
    #[arg(short, long)]
    exclude: Vec<String>,

    /// Include hidden files and directories
    #[arg(long)]
    hidden: bool,

    /// Don't skip lockfiles and minified/source-map files
    #[arg(long)]
    no_default_excludes: bool,

    /// Exit with code 1 if any warnings are reported
    #[arg(long)]
    fail_on_warn: bool,
}

fn main() -> Result<ExitCode> {
    let args = Args::parse();
    let paths = walk::collect_files(
        &args.path,
        &walk::WalkOptions {
            include: &args.include,
            exclude: &args.exclude,
            hidden: args.hidden,
            default_excludes: !args.no_default_excludes,
        },
    )?;

    let files: Vec<_> = paths
        .par_iter()
        .filter_map(|p| {
            let display = p.strip_prefix(&args.path).unwrap_or(p).to_path_buf();
            count::analyze(p, display, args.tokenizer, !args.no_functions)
        })
        .collect();

    let limits = Thresholds {
        max_file_tokens: args.max_file_tokens,
        max_file_lines: args.max_file_lines,
        max_fn_lines: args.max_fn_lines,
    };
    let tokenizer = format!("{:?}", args.tokenizer).to_lowercase();
    let report = Report::build(files, args.sort, &limits, tokenizer);

    let rendered = match args.format {
        Format::Table => report.to_table(args.top),
        Format::Json => report.to_json(),
        Format::Md => report.to_markdown(args.top),
    };
    println!("{rendered}");

    Ok(if args.fail_on_warn && !report.warnings.is_empty() {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    })
}
