//! Per-file measurement: token counts and code/comment/blank line breakdown;
//! generated files are recognized and left out.

mod chunk;
mod comments;
mod file_stats;
mod generated;
mod pretokenizer;
mod tokenizer;

pub use file_stats::{Analysis, AnalysisRun, AnalyzeInput, FileStats, SourceInput};
pub use tokenizer::Tokenizer;
