use std::path::{Component, Path};

use anyhow::{Context, Result};

use super::glob::GlobHelpers;

/// The `require_test` path templates of one `[[files]]` rule: `{stem}`, `{name}` and `{dir}`
/// are replaced from the checked file, the rest is a glob relative to the analyzed root.
#[derive(Debug)]
pub struct TestTemplates(Vec<String>);

impl TestTemplates {
    pub fn new(templates: &[String]) -> Result<Self> {
        for template in templates {
            let sample = Self::fill(template, ("x", "x.x", "x"));
            GlobHelpers::path_matcher(&sample)
                .with_context(|| format!("invalid require_test template {template:?}"))?;
        }
        Ok(Self(templates.to_vec()))
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Each template with the placeholders of `file` filled in (glob-escaped).
    pub fn resolve(&self, file: &Path) -> Vec<String> {
        let name = file
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let stem = file
            .file_stem()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let dir = Self::dir_of(file);
        let parts = (
            globset::escape(&stem),
            globset::escape(&name),
            globset::escape(&dir),
        );
        self.0
            .iter()
            .map(|t| Self::fill(t, (&parts.0, &parts.1, &parts.2)))
            .collect()
    }

    /// `(stem, name, dir)`; an empty `dir` (the root) drops the `/` that follows it.
    fn fill(template: &str, parts: (&str, &str, &str)) -> String {
        let (stem, name, dir) = parts;
        let with_dir = if dir.is_empty() {
            template.replace("{dir}/", "").replace("{dir}", "")
        } else {
            template.replace("{dir}", dir)
        };
        let filled = with_dir.replace("{stem}", stem).replace("{name}", name);
        filled.strip_prefix("./").unwrap_or(&filled).to_string()
    }

    fn dir_of(file: &Path) -> String {
        let parent = file.parent().unwrap_or(Path::new(""));
        parent
            .components()
            .filter_map(|c| match c {
                Component::Normal(s) => Some(s.to_string_lossy().into_owned()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("/")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fills_placeholders_and_escapes() {
        let t = TestTemplates::new(&[
            "tests/{stem}.rs".to_string(),
            "{dir}/{stem}_test.{name}".to_string(),
        ])
        .unwrap();
        assert_eq!(
            t.resolve(Path::new("src/a/b.rs")),
            vec!["tests/b.rs", "src/a/b_test.b.rs"]
        );
        assert_eq!(t.resolve(Path::new("main.rs"))[1], "main_test.main.rs");
        assert_eq!(t.resolve(Path::new("src/x[1].rs"))[0], "tests/x[[]1[]].rs");
        assert!(TestTemplates::new(&["tests/[{stem}.rs".to_string()]).is_err());
    }
}
