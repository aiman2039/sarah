use std::path::Path;
use std::sync::Arc;

use super::LimitRules;
use crate::config::patterns::LinePatterns;
use crate::units::{Chars, LineSpan};

/// The rules of one path that change what per-file analysis measures.
#[derive(Clone, Copy, Debug, Default)]
pub struct AnalysisRules<'a> {
    /// Strip inline unit tests (`#[cfg(test)]`, Zig `test`) before counting.
    pub exclude_inline_tests: bool,
    /// Find inline unit tests: to strip them, for `require_test` or for a folder test ratio.
    pub find_inline_tests: bool,
    /// `forbid_patterns` that apply to this path.
    pub forbid_patterns: Option<&'a LinePatterns>,
    /// `max_line_length` of this path; lines over it are summarized.
    pub max_line_length: Option<Chars>,
}

/// `AnalysisRules` without borrows: equal keys give equal analyses (for cache stamps).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnalysisKey {
    pub exclude_inline_tests: bool,
    pub find_inline_tests: bool,
    pub forbid_patterns: Option<Arc<str>>,
    pub max_line_length: Option<Chars>,
}

impl AnalysisRules<'_> {
    /// The inline-test `spans` that measurements leave out: all when they are stripped.
    pub fn stripped<'s>(&self, spans: &'s [LineSpan]) -> &'s [LineSpan] {
        if self.exclude_inline_tests {
            spans
        } else {
            &[]
        }
    }

    pub fn key(&self) -> AnalysisKey {
        AnalysisKey {
            exclude_inline_tests: self.exclude_inline_tests,
            find_inline_tests: self.find_inline_tests,
            forbid_patterns: self.forbid_patterns.map(LinePatterns::key),
            max_line_length: self.max_line_length,
        }
    }
}

impl LimitRules {
    /// What analysis of `path` needs; `folder_test_ratio`: a folder rule sets a test ratio.
    pub fn analysis_for(&self, path: &Path, folder_test_ratio: bool) -> AnalysisRules<'_> {
        let limits = self.for_path(path);
        let exclude_inline_tests = limits.flags.exclude_inline_tests;
        AnalysisRules {
            exclude_inline_tests,
            find_inline_tests: exclude_inline_tests
                || folder_test_ratio
                || self.require_test_for(path).is_some(),
            forbid_patterns: self.forbid_patterns_for(path),
            max_line_length: limits.line_length,
        }
    }
}
