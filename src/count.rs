use std::path::{Path, PathBuf};

use clap::ValueEnum;
use rayon::prelude::*;
use serde::Serialize;
use tokei::{Config, LanguageType};

use crate::functions::{self, FnStats};

/// Claude's tokenizer isn't public; the `o200k`/`cl100k` encodings (via `bpe-openai`,
/// which matches `tiktoken` exactly) are a close approximation.
#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum Tokenizer {
    O200k,
    Cl100k,
}

/// Files larger than this are tokenized in chunks across the rayon pool, so one
/// huge file doesn't leave every other core idle.
const CHUNK_BYTES: usize = 256 * 1024;

impl Tokenizer {
    pub fn count(self, text: &str) -> usize {
        self.count_chunked(text, CHUNK_BYTES)
    }

    fn encoder(self) -> &'static bpe_openai::Tokenizer {
        match self {
            Self::O200k => bpe_openai::o200k_base(),
            Self::Cl100k => bpe_openai::cl100k_base(),
        }
    }

    fn count_chunked(self, text: &str, chunk_bytes: usize) -> usize {
        let bpe = self.encoder();
        if text.len() <= chunk_bytes {
            return bpe.count(text);
        }
        split_at_token_boundaries(text, chunk_bytes)
            .into_par_iter()
            .map(|chunk| bpe.count(chunk))
            .sum()
    }
}

/// Split `text` into pieces of roughly `target` bytes, cutting only where the
/// tiktoken pre-tokenizer always splits, so the summed count is exact.
///
/// A cut goes right after a `\n` whose following spaces/tabs end in a plain
/// ASCII non-whitespace character. Then the newline ends its pre-token: the
/// `\s*[\r\n]+` alternative stops at the last newline of a whitespace run, and
/// no letter, number or whitespace pattern crosses into the next line. `/` right
/// after the newline is excluded because o200k's punctuation pattern
/// `[^\s\p{L}\p{N}]+[\r\n/]*` can absorb it.
fn split_at_token_boundaries(text: &str, target: usize) -> Vec<&str> {
    let mut chunks = Vec::new();
    let mut rest = text;
    while rest.len() > target {
        let Some(cut) = next_cut(rest.as_bytes(), target) else {
            break;
        };
        let Some((head, tail)) = rest.split_at_checked(cut) else {
            break;
        };
        chunks.push(head);
        rest = tail;
    }
    chunks.push(rest);
    chunks
}

/// First safe cut at or after `from`: the index just past a qualifying `\n`.
fn next_cut(bytes: &[u8], from: usize) -> Option<usize> {
    let mut i = from;
    loop {
        let newline = bytes.get(i..)?.iter().position(|&b| b == b'\n')?;
        let after = i.checked_add(newline)?.checked_add(1)?;
        let mut j = after;
        while matches!(bytes.get(j), Some(b' ' | b'\t')) {
            j = j.checked_add(1)?;
        }
        match bytes.get(j) {
            Some(b'/') if j == after => {}
            Some(&b) if b.is_ascii() && !b.is_ascii_whitespace() => return Some(after),
            Some(_) => {}
            None => return None,
        }
        i = after;
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

    /// Cutting at every allowed boundary must give exactly the unsplit count.
    #[test]
    fn chunked_count_is_exact() {
        let samples = [
            // Code: comments at column 0 and after punctuation, CRLF, tabs.
            "fn a() {\n    1;\n}\n// c\n/* x */\nfn b() {}\r\n\tlet x = 1;\r\n}\n//\n",
            // Pretty JSON: every line indented, blank lines holding spaces.
            "{\n  \"a\": [\n    1,\n    2\n  ],\n   \n  \"b\": \"x\"\n}\n",
            // Whitespace runs spanning several newlines, trailing spaces.
            "x  \n  \n\n   y\n\n\nz   \n\t\n w\n",
            // Non-ASCII after a newline, numbers, punctuation runs.
            "é\nñ 123456\n...\n!!!\n\u{a0}x\n12\n34\n'''s\n's\n",
        ];
        for tokenizer in [Tokenizer::O200k, Tokenizer::Cl100k] {
            for text in samples {
                let whole = tokenizer.encoder().count(text);
                assert!(split_at_token_boundaries(text, 1).len() > 2);
                assert_eq!(tokenizer.count_chunked(text, 1), whole, "{text:?}");
            }
        }
    }

    #[test]
    fn no_cut_inside_multi_newline_whitespace() {
        // The only safe cut is before "y"; the first newline belongs to "\n  \n".
        assert_eq!(
            split_at_token_boundaries("a\n  \ny", 1),
            vec!["a\n  \n", "y"]
        );
        assert_eq!(split_at_token_boundaries("a\n/b", 1), vec!["a\n/b"]);
    }
}
