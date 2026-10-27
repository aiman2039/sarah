# Local patch on bpe-openai 0.3.2

Source: crates.io `bpe-openai` 0.3.2 (github/rust-gems), unmodified except `src/lib.rs`.

Change: `Pretokenizer` owns a `regex_automata` cache pool, and each `Splits`
iterator takes one cache for its whole lifetime (`Regex::search_with`) instead of
calling `Regex::find`, which takes a cache from the regex's shared pool once per
pre-token piece. Only the pool's owner thread has a lock-free fast path, so with
many threads tokenizing at once most time went to that pool's mutex.

Upstream PR: see sarah's README / git history. Remove this vendored copy and the
`[patch.crates-io]` entry in Cargo.toml once a release includes the fix.
