use std::panic::{RefUnwindSafe, UnwindSafe};
use std::sync::LazyLock;

use bpe_openai::byte_pair_encoding::BytePairEncoding;
use regex_automata::util::pool::Pool;
use regex_automata::{Anchored, Input, meta::Cache, meta::Regex};

/// tiktoken's pre-tokenizer, as `bpe-openai` 0.3.2 implements it (`src/lib.rs`), with one
/// change: each `count` call holds one regex cache for the whole text. `bpe-openai` takes
/// a cache from the regex's shared pool for every piece; only the pool's owner thread
/// has a lock-free path, so parallel counting spent most of its time on that mutex.
///
/// The negative look-ahead `\s+(?!\S)` is rewritten as `\s+\s` with the last character
/// dropped from the match (pattern 1), plus `\s+$` in pattern 0 for end of text.
/// `pretokenizer_matches_bpe_openai` checks the counts against `bpe-openai`.
pub(super) struct Pretokenizer {
    regex: Regex,
    caches: Pool<Cache, CachePoolFn>,
}

type CachePoolFn = Box<dyn Fn() -> Cache + Send + Sync + UnwindSafe + RefUnwindSafe>;

const CL100K_PATTERN: &str = "(?i:'s|'t|'re|'ve|'m|'ll|'d)|[^\\r\\n\\p{L}\\p{N}]?\\p{L}+|\\p{N}{1,3}| ?[^\\s\\p{L}\\p{N}]+[\\r\\n]*|\\s*[\\r\\n]+|\\s+$";

const O200K_PATTERN: &str = concat!(
    "[^\\r\\n\\p{L}\\p{N}]?[\\p{Lu}\\p{Lt}\\p{Lm}\\p{Lo}\\p{M}]*[\\p{Ll}\\p{Lm}\\p{Lo}\\p{M}]+(?i:'s|'t|'re|'ve|'m|'ll|'d)?",
    "|[^\\r\\n\\p{L}\\p{N}]?[\\p{Lu}\\p{Lt}\\p{Lm}\\p{Lo}\\p{M}]+[\\p{Ll}\\p{Lm}\\p{Lo}\\p{M}]*(?i:'s|'t|'re|'ve|'m|'ll|'d)?",
    "|\\p{N}{1,3}",
    "| ?[^\\s\\p{L}\\p{N}]+[\\r\\n/]*",
    "|\\s*[\\r\\n]+",
    "|\\s+$",
);

/// Index of the look-ahead pattern (`\s+\s`) in `Regex::new_many`.
const LOOKAHEAD_PATTERN: usize = 1;

static CL100K: LazyLock<Option<Pretokenizer>> = LazyLock::new(|| Pretokenizer::new(CL100K_PATTERN));
static O200K: LazyLock<Option<Pretokenizer>> = LazyLock::new(|| Pretokenizer::new(O200K_PATTERN));

impl Pretokenizer {
    pub(super) fn cl100k() -> Option<&'static Self> {
        CL100K.as_ref()
    }

    pub(super) fn o200k() -> Option<&'static Self> {
        O200K.as_ref()
    }

    fn new(first: &str) -> Option<Self> {
        let regex = Regex::new_many(&[first, "\\s+\\s", "\\s+"]).ok()?;
        let for_pool = regex.clone();
        let caches: Pool<Cache, CachePoolFn> = Pool::new(Box::new(move || for_pool.create_cache()));
        Some(Self { regex, caches })
    }

    /// Token count of `text`. `None` if the text splits in a way `bpe-openai` would
    /// reject (it panics there); the caller then counts with `bpe-openai` itself.
    pub(super) fn count(&self, bpe: &BytePairEncoding, text: &str) -> Option<usize> {
        let mut cache = self.caches.get();
        let mut total: usize = 0;
        let mut rest = text;
        while !rest.is_empty() {
            let input = Input::new(rest).anchored(Anchored::Yes);
            let found = self.regex.search_with(&mut cache, &input)?;
            let mut end = found.end();
            if found.pattern().as_usize() == LOOKAHEAD_PATTERN {
                let lookahead = rest.get(..end)?.chars().next_back()?;
                end = end.checked_sub(lookahead.len_utf8())?;
            }
            if end == 0 {
                return None;
            }
            let (piece, tail) = rest.split_at_checked(end)?;
            total = total.saturating_add(bpe.count(piece.as_bytes()));
            rest = tail;
        }
        Some(total)
    }
}
