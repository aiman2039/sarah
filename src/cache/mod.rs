//! Thread-safe in-memory cache of `FileStats` keyed by `(realpath, mtime, size, options)`.

mod key;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use self::key::CacheStamp;
use crate::count::{Analysis, AnalyzeInput, FileStats};

#[derive(Default)]
pub struct ScanCache {
    entries: Mutex<HashMap<PathBuf, (CacheStamp, Analysis)>>,
}

impl ScanCache {
    pub fn new() -> Self {
        Self::default()
    }

    /// Return cached `Analysis` when `mtime`, `len` and options match; else measure and store.
    pub fn analyze(&self, input: AnalyzeInput) -> Analysis {
        let Some(stamp) = CacheStamp::of(&input) else {
            return FileStats::analyze(input);
        };
        let real = input.entry.real.clone();
        let display = input.entry.display.clone();
        if let Some(mut hit) = self.lookup(&real, &stamp) {
            if let Analysis::Counted(stats) = &mut hit {
                stats.path = display;
            }
            return hit;
        }
        let fresh = FileStats::analyze(input);
        self.store(real, stamp, fresh.clone());
        fresh
    }

    pub fn file_sig(path: &Path) -> Option<(u128, u64)> {
        CacheStamp::file_sig(path)
    }

    fn lookup(&self, real: &Path, stamp: &CacheStamp) -> Option<Analysis> {
        let guard = self
            .entries
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let (saved_stamp, saved_analysis) = guard.get(real)?;
        (saved_stamp == stamp).then(|| saved_analysis.clone())
    }

    fn store(&self, real: PathBuf, stamp: CacheStamp, analysis: Analysis) {
        let mut guard = self
            .entries
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        guard.insert(real, (stamp, analysis));
    }
}
