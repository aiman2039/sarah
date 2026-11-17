use std::path::Path;
use std::time::UNIX_EPOCH;

use crate::config::AnalysisKey;
use crate::count::{AnalysisRun, AnalyzeInput};

/// File signature plus everything that changes the analysis of the file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct CacheStamp {
    pub modified_nanos: u128,
    pub len: u64,
    pub run: AnalysisRun,
    pub rules: AnalysisKey,
}

impl CacheStamp {
    pub(super) fn of(input: &AnalyzeInput<'_>) -> Option<Self> {
        let (modified_nanos, len) = Self::file_sig(&input.entry.real)?;
        Some(Self {
            modified_nanos,
            len,
            run: input.run,
            rules: input.rules.key(),
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
