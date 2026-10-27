use clap::ValueEnum;
use derive_more::Display;
use rayon::prelude::*;

use super::chunk::ChunkHelpers;
use super::pretokenizer::Pretokenizer;
use crate::units::Tokens;

/// Claude's tokenizer isn't public; the `o200k`/`cl100k` encodings (via `bpe-openai`,
/// which matches `tiktoken` exactly) are a close approximation.
#[derive(Clone, Copy, Debug, Display, ValueEnum)]
pub enum Tokenizer {
    #[display("o200k")]
    O200k,
    #[display("cl100k")]
    Cl100k,
}

/// Files larger than this are tokenized in chunks across the rayon pool, so one
/// huge file doesn't leave every other core idle.
const CHUNK_BYTES: usize = 256 * 1024;

impl Tokenizer {
    pub fn count(self, text: &str) -> Tokens {
        self.count_chunked(text, CHUNK_BYTES)
    }

    fn encoder(self) -> &'static bpe_openai::Tokenizer {
        match self {
            Self::O200k => bpe_openai::o200k_base(),
            Self::Cl100k => bpe_openai::cl100k_base(),
        }
    }

    fn pretokenizer(self) -> Option<&'static Pretokenizer> {
        match self {
            Self::O200k => Pretokenizer::o200k(),
            Self::Cl100k => Pretokenizer::cl100k(),
        }
    }

    /// Count with sarah's contention-free pre-tokenizer; fall back to `bpe-openai`'s
    /// own `count` if it can't handle the text.
    fn count_piece(self, text: &str) -> Tokens {
        let encoder = self.encoder();
        let fast = self
            .pretokenizer()
            .and_then(|pre| pre.count(&encoder.bpe, text));
        Tokens(fast.unwrap_or_else(|| encoder.count(text)))
    }

    fn count_chunked(self, text: &str, chunk_bytes: usize) -> Tokens {
        if text.len() <= chunk_bytes {
            return self.count_piece(text);
        }
        ChunkHelpers::split_at_token_boundaries(text, chunk_bytes)
            .into_par_iter()
            .map(|chunk| self.count_piece(chunk))
            .reduce(Tokens::default, Tokens::saturating_add)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_count_known_string() {
        assert_eq!(Tokenizer::O200k.count("hello world"), Tokens(2));
        assert_eq!(Tokenizer::Cl100k.count("hello world"), Tokens(2));
        assert_eq!(Tokenizer::O200k.count(""), Tokens(0));
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
                let whole = Tokens(tokenizer.encoder().count(text));
                assert!(ChunkHelpers::split_at_token_boundaries(text, 1).len() > 2);
                assert_eq!(tokenizer.count_chunked(text, 1), whole, "{text:?}");
            }
        }
    }

    /// sarah's pre-tokenizer must count exactly like `bpe-openai` (and so `tiktoken`).
    #[test]
    fn pretokenizer_matches_bpe_openai() {
        let sources = [
            include_str!("tokenizer.rs"),
            include_str!("chunk.rs"),
            include_str!("../walk/walker.rs"),
            include_str!("../report/render/markdown.rs"),
        ]
        .concat();
        let tricky = "I'm here, they'll SAY 'S're\n\n  \n\t x  \r\n12345 ...!!! /path//x\n\
             héllo Ωmega 日本語 émoji 🙂 \u{a0}nbsp\u{2003}em  trailing   \n\n\n";
        for tokenizer in [Tokenizer::O200k, Tokenizer::Cl100k] {
            let pre = tokenizer.pretokenizer().unwrap();
            let encoder = tokenizer.encoder();
            for text in [sources.as_str(), tricky, "", " ", "\n", "  \n  ", "a"] {
                assert_eq!(
                    pre.count(&encoder.bpe, text),
                    Some(encoder.count(text)),
                    "{tokenizer:?} {text:?}"
                );
            }
        }
    }
}
