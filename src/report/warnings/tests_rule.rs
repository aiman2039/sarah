use std::collections::HashSet;
use std::path::Path;

use globset::GlobSetBuilder;

use crate::config::{GlobHelpers, LimitRules};
use crate::count::FileStats;
use crate::report::warning::{Warning, WarningKind};

/// Characters that make a resolved template a glob rather than one literal path.
const GLOB_CHARS: &[char] = &['*', '?', '[', ']', '{', '}'];

/// One file under a `require_test` rule, with its resolved templates.
struct Pending<'a> {
    file: &'a FileStats,
    resolved: Vec<String>,
    found: bool,
}

impl Warning {
    /// Files under a `require_test` rule with no matching test file and no inline test.
    pub fn for_missing_tests(files: &[FileStats], rules: &LimitRules) -> Vec<Self> {
        let mut pending = Self::pending_tests(files, rules);
        if pending.is_empty() {
            return Vec::new();
        }
        Self::match_literal_tests(files, &mut pending);
        Self::match_glob_tests(files, &mut pending);
        pending
            .into_iter()
            .filter(|p| !p.found)
            .map(|p| {
                Self::file(
                    p.file,
                    WarningKind::MissingTest,
                    format!(
                        "no test file matches {} and no inline test — add a test",
                        p.resolved.join(", ")
                    ),
                )
            })
            .collect()
    }

    /// Files that a `require_test` rule covers and that have no inline test.
    fn pending_tests<'a>(files: &'a [FileStats], rules: &LimitRules) -> Vec<Pending<'a>> {
        files
            .iter()
            .filter(|f| !f.inline_tests.has_tests())
            .filter_map(|file| {
                let templates = rules.require_test_for(&file.path)?;
                Some(Pending {
                    file,
                    resolved: templates.resolve(&file.path),
                    found: false,
                })
            })
            .collect()
    }

    /// Literal templates: one set lookup each.
    fn match_literal_tests(files: &[FileStats], pending: &mut [Pending<'_>]) {
        let walked: HashSet<&Path> = files.iter().map(|f| f.path.as_path()).collect();
        for p in pending {
            p.found = p.resolved.iter().any(|pattern| {
                let candidate = Path::new(pattern);
                !pattern.contains(GLOB_CHARS)
                    && candidate != p.file.path
                    && walked.contains(candidate)
            });
        }
    }

    /// Glob templates: one set of every pattern, matched once against each walked path.
    fn match_glob_tests(files: &[FileStats], pending: &mut [Pending<'_>]) {
        let mut builder = GlobSetBuilder::new();
        let mut owners = Vec::new();
        for (idx, p) in pending.iter().enumerate().filter(|(_, p)| !p.found) {
            let globs = p.resolved.iter().filter(|t| t.contains(GLOB_CHARS));
            for glob in globs.filter_map(|t| GlobHelpers::path_glob(t).ok()) {
                builder.add(glob);
                owners.push(idx);
            }
        }
        let Ok(set) = builder.build() else {
            return;
        };
        for path in files.iter().map(|f| &f.path) {
            for owner in set.matches(path).into_iter().filter_map(|i| owners.get(i)) {
                if let Some(p) = pending.get_mut(*owner)
                    && p.file.path != *path
                {
                    p.found = true;
                }
            }
        }
    }
}
