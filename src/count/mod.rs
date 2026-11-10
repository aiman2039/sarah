//! Per-file measurement: token counts and code/comment/blank line breakdown;
//! generated files are recognized and left out.

mod chunk;
mod comments;
mod file_stats;
mod generated;
mod pretokenizer;
mod tokenizer;

#[cfg(test)]
pub use comments::CommentInfo;
pub use file_stats::{Analysis, AnalyzeInput, FileStats, SourceInput};
pub use tokenizer::Tokenizer;
