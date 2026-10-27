use clap::ValueEnum;
use rayon::prelude::*;

use super::chunk::ChunkHelpers;
use crate::units::Tokens;

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
    pub fn count(self, text: &str) -> Tokens {
        self.count_chunked(text, CHUNK_BYTES)
    }

    fn encoder(self) -> &'static bpe_openai::Tokenizer {
        match self {
            Self::O200k => bpe_openai::o200k_base(),
            Self::Cl100k => bpe_openai::cl100k_base(),
        }
    }

    fn count_chunked(self, text: &str, chunk_bytes: usize) -> Tokens {
        let bpe = self.encoder();
        if text.len() <= chunk_bytes {
            return Tokens(bpe.count(text));
        }
        ChunkHelpers::split_at_token_boundaries(text, chunk_bytes)
            .into_par_iter()
            .map(|chunk| Tokens(bpe.count(chunk)))
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
}
