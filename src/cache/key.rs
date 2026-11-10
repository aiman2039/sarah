use std::path::Path;
use std::time::UNIX_EPOCH;

use crate::count::AnalyzeInput;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct CacheStamp {
    pub modified_nanos: u128,
    pub len: u64,
    pub tokenizer: String,
    pub flags: u8,
}

impl CacheStamp {
    pub(super) fn of(input: &AnalyzeInput) -> Option<Self> {
        let meta = std::fs::metadata(&input.entry.real).ok()?;
        let modified = meta.modified().ok()?;
        let modified_nanos = modified.duration_since(UNIX_EPOCH).ok()?.as_nanos();
        let flags = u8::from(input.with_functions)
            | (u8::from(input.skip_generated) << 1)
            | (u8::from(input.exclude_inline_tests) << 2);
        Some(Self {
            modified_nanos,
            len: meta.len(),
            tokenizer: input.tokenizer.to_string(),
            flags,
        })
    }

    pub fn file_sig(path: &Path) -> Option<(u128, u64)> {
        let meta = std::fs::metadata(path).ok()?;
        let nanos = meta
            .modified()
            .ok()?
            .duration_since(UNIX_EPOCH)
            .ok()?
            .as_nanos();
        Some((nanos, meta.len()))
    }
}
