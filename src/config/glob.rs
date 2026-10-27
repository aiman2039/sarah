use std::path::Path;

use anyhow::{Context, Result};
use globset::{Glob, GlobBuilder, GlobMatcher};

/// Stateless glob constructors shared by file and folder rules.
pub struct GlobHelpers;

impl GlobHelpers {
    /// Path glob: `*` stays inside one path component, `**` crosses them (gitignore-style).
    pub fn path_matcher(pattern: &str) -> Result<GlobMatcher> {
        Ok(GlobBuilder::new(pattern)
            .literal_separator(true)
            .build()
            .with_context(|| format!("invalid glob {pattern:?}"))?
            .compile_matcher())
    }
}

/// A folder glob. The analyzed root is the folder `.`; only `.` and `**` select it.
#[derive(Debug)]
pub struct DirPattern {
    pattern: String,
    matcher: GlobMatcher,
}

impl DirPattern {
    pub fn new(pattern: &str) -> Result<Self> {
        Ok(Self {
            pattern: pattern.to_string(),
            matcher: GlobHelpers::path_matcher(pattern)?,
        })
    }

    pub fn matches(&self, dir: &Path) -> bool {
        if dir == Path::new(".") {
            return matches!(self.pattern.as_str(), "." | "**");
        }
        self.matcher.is_match(dir)
    }
}

/// Globs matched against one file or folder name (no `/`).
#[derive(Debug)]
pub struct NamePatterns(Vec<(String, GlobMatcher)>);

impl NamePatterns {
    pub fn new(patterns: &[String]) -> Result<Self> {
        patterns
            .iter()
            .map(|p| {
                let glob = Glob::new(p).with_context(|| format!("invalid glob {p:?}"))?;
                Ok((p.clone(), glob.compile_matcher()))
            })
            .collect::<Result<_>>()
            .map(Self)
    }

    /// The first pattern that matches `name`.
    pub fn find(&self, name: &str) -> Option<&str> {
        self.0
            .iter()
            .find(|(_, m)| m.is_match(name))
            .map(|(p, _)| p.as_str())
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &GlobMatcher)> {
        self.0.iter().map(|(p, m)| (p.as_str(), m))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dir_pattern_semantics() {
        let src = DirPattern::new("src/**").unwrap();
        assert!(src.matches(Path::new("src/a")));
        assert!(src.matches(Path::new("src/a/b")));
        assert!(!src.matches(Path::new("src")));
        assert!(!src.matches(Path::new(".")));

        let one = DirPattern::new("*").unwrap();
        assert!(one.matches(Path::new("src")));
        assert!(!one.matches(Path::new("src/a")));
        assert!(!one.matches(Path::new(".")));

        assert!(DirPattern::new("**").unwrap().matches(Path::new(".")));
        assert!(DirPattern::new(".").unwrap().matches(Path::new(".")));
    }
}
