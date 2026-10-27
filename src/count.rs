use std::path::{Path, PathBuf};

use clap::ValueEnum;
use serde::Serialize;
use tiktoken_rs::{cl100k_base_singleton, o200k_base_singleton};
use tokei::{Config, LanguageType};

use crate::functions::{self, FnStats};

/// Claude's tokenizer isn't public; these `tiktoken` encodings are a close approximation.
#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum Tokenizer {
    O200k,
    Cl100k,
}

impl Tokenizer {
    pub fn count(self, text: &str) -> usize {
        let bpe = match self {
            Self::O200k => o200k_base_singleton(),
            Self::Cl100k => cl100k_base_singleton(),
        };
        bpe.encode_ordinary(text).len()
    }
}

#[derive(Debug, Serialize)]
pub struct FileStats {
    pub path: PathBuf,
    pub language: String,
    pub bytes: usize,
    pub lines: usize,
    pub code: usize,
    pub comments: usize,
    pub blanks: usize,
    pub tokens: usize,
    /// `None` when function extraction isn't supported for this language or was disabled.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub functions: Option<Vec<FnStats>>,
}

/// Returns `None` for files that aren't valid UTF-8 (binaries etc.).
pub fn analyze(
    path: &Path,
    display: PathBuf,
    tokenizer: Tokenizer,
    with_functions: bool,
) -> Option<FileStats> {
    let text = std::fs::read_to_string(path).ok()?;
    let config = Config::default();
    let lang = LanguageType::from_path(path, &config);
    let (code, comments, blanks) = lang.map_or_else(
        || {
            let blanks = text.lines().filter(|l| l.trim().is_empty()).count();
            (text.lines().count().saturating_sub(blanks), 0, blanks)
        },
        |l| {
            let s = l.parse_from_str(&text, &config).summarise();
            (s.code, s.comments, s.blanks)
        },
    );
    Some(FileStats {
        language: lang.map_or("Other".to_string(), |l| l.name().to_string()),
        bytes: text.len(),
        lines: text.lines().count(),
        code,
        comments,
        blanks,
        tokens: tokenizer.count(&text),
        functions: with_functions
            .then(|| functions::extract(path, &text, tokenizer))
            .flatten(),
        path: display,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_count_known_string() {
        assert_eq!(Tokenizer::O200k.count("hello world"), 2);
        assert_eq!(Tokenizer::Cl100k.count("hello world"), 2);
        assert_eq!(Tokenizer::O200k.count(""), 0);
    }
}
