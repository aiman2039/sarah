use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use globset::GlobSet;

use super::rules::{FolderRules, Policy};
use crate::config::{DEFAULT_TEST_GLOBS, GlobHelpers};
use crate::count::FileStats;
use crate::functions::AstHelpers;
use crate::report::{Warning, WarningKind};
use crate::units::{Lines, Ratio};

/// Globs (relative to the analyzed root) that mark whole files as test code.
#[derive(Debug)]
pub struct TestPaths(GlobSet);

/// Code lines of one folder subtree, split into test and non-test code.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct TestTotals {
    test: Lines,
    non_test: Lines,
}

impl TestPaths {
    pub fn new(globs: &[String]) -> Result<Self> {
        let set = GlobHelpers::path_set(globs.iter().map(String::as_str));
        Ok(Self(set.context("invalid test_paths")?))
    }

    pub(super) fn default_set() -> Result<Self> {
        let set = GlobHelpers::path_set(DEFAULT_TEST_GLOBS.iter().copied());
        Ok(Self(set.context("invalid default test_paths")?))
    }

    fn is_test(&self, path: &Path) -> bool {
        self.0.is_match(path)
    }
}

impl FolderRules {
    /// Use `globs` (the `test_paths` key) instead of the default test-file globs.
    pub fn with_test_paths(mut self, globs: Option<&[String]>) -> Result<Self> {
        if let Some(globs) = globs {
            self.test_paths = TestPaths::new(globs).context("in test_paths of sarah.toml")?;
        }
        Ok(self)
    }

    /// `min_test_ratio` / `max_test_ratio` violations, folder by folder in path order.
    pub fn check_test_ratio(&self, files: &[FileStats]) -> Vec<Warning> {
        if !self.has_test_ratio() {
            return Vec::new();
        }
        TestTotals::by_dir(files, &self.test_paths)
            .into_iter()
            .filter_map(|(dir, totals)| {
                let policy = self.policy_for(&dir)?;
                totals.warning(&dir, &policy)
            })
            .collect()
    }
}

impl TestTotals {
    fn by_dir(files: &[FileStats], paths: &TestPaths) -> BTreeMap<PathBuf, Self> {
        let mut out: BTreeMap<PathBuf, Self> = BTreeMap::new();
        for f in files {
            let own = Self::of_file(f, paths);
            let parent = f.path.parent().unwrap_or(Path::new(""));
            for dir in parent.ancestors() {
                let key = if dir.as_os_str().is_empty() {
                    PathBuf::from(".")
                } else {
                    dir.to_path_buf()
                };
                let entry = out.entry(key).or_default();
                entry.test = entry.test.saturating_add(own.test);
                entry.non_test = entry.non_test.saturating_add(own.non_test);
            }
        }
        out
    }

    /// Only source files count (languages with function extraction), not config or docs.
    fn of_file(f: &FileStats, paths: &TestPaths) -> Self {
        if !AstHelpers::is_code_file(&f.path) {
            return Self::default();
        }
        if paths.is_test(&f.path) {
            return Self {
                test: f.code,
                non_test: Lines(0),
            };
        }
        Self {
            test: f.inline_tests.code,
            non_test: f.inline_tests.non_test_code(f.code),
        }
    }

    fn warning(self, dir: &Path, policy: &Policy<'_>) -> Option<Warning> {
        if self.non_test == Lines(0) {
            return None;
        }
        let ratio = Ratio {
            part: self.test.0,
            whole: self.non_test.0,
        };
        let percent = ratio.percent();
        let lines = format!("{} test / {} non-test code lines", self.test, self.non_test);
        if let Some(min) = policy.min_test_ratio.filter(|min| ratio.below(*min)) {
            return Some(Warning::new(
                dir.to_path_buf(),
                WarningKind::FolderLowTestRatio,
                format!("test ratio {percent}% ({lines}) (<{min}%) — add tests"),
            ));
        }
        let max = policy.max_test_ratio.filter(|max| ratio.above(*max))?;
        Some(Warning::new(
            dir.to_path_buf(),
            WarningKind::FolderHighTestRatio,
            format!("test ratio {percent}% ({lines}) (>{max}%) — trim redundant tests"),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_globs_mark_test_files() {
        let paths = TestPaths::default_set().unwrap();
        for test in [
            "tests/a.rs",
            "src/tests/b.rs",
            "a_test.go",
            "pkg/test_x.py",
            "a.spec.ts",
        ] {
            assert!(paths.is_test(Path::new(test)), "{test}");
        }
        for prod in ["src/a.rs", "src/latest.rs", "contest/a.py"] {
            assert!(!paths.is_test(Path::new(prod)), "{prod}");
        }
    }
}
