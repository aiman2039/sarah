use derive_more::Display;
use serde::Deserialize;

/// Naming convention for file stems and folder names.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Display, Deserialize)]
pub enum Naming {
    #[serde(rename = "snake_case")]
    #[display("snake_case")]
    Snake,
    #[serde(rename = "kebab-case")]
    #[display("kebab-case")]
    Kebab,
    #[serde(rename = "camelCase")]
    #[display("camelCase")]
    Camel,
    #[serde(rename = "PascalCase")]
    #[display("PascalCase")]
    Pascal,
}

impl Naming {
    /// Checks a file or folder name: the part before the first `.`. Dotfiles always pass,
    /// and snake_case allows leading/trailing `_` (`__init__.py`).
    pub fn accepts(self, name: &str) -> bool {
        if name.starts_with('.') {
            return true;
        }
        let stem = name.split('.').next().unwrap_or(name);
        let lower_or_digit = |c: char| c.is_ascii_lowercase() || c.is_ascii_digit();
        match self {
            Self::Snake => {
                let core = stem.trim_matches('_');
                !core.is_empty() && core.chars().all(|c| lower_or_digit(c) || c == '_')
            }
            Self::Kebab => {
                !stem.is_empty()
                    && !stem.starts_with('-')
                    && !stem.ends_with('-')
                    && stem.chars().all(|c| lower_or_digit(c) || c == '-')
            }
            Self::Camel => Self::alnum_after(stem, |c| c.is_ascii_lowercase()),
            Self::Pascal => Self::alnum_after(stem, |c| c.is_ascii_uppercase()),
        }
    }

    /// The first char passes `first`; the rest are ASCII letters or digits.
    fn alnum_after(stem: &str, first: impl Fn(char) -> bool) -> bool {
        let mut chars = stem.chars();
        chars.next().is_some_and(first) && chars.all(|c| c.is_ascii_alphanumeric())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conventions() {
        assert!(Naming::Snake.accepts("file_stats.rs"));
        assert!(Naming::Snake.accepts("__init__.py"));
        assert!(Naming::Snake.accepts(".gitignore"));
        assert!(!Naming::Snake.accepts("FileStats.rs"));
        assert!(!Naming::Snake.accepts("file-stats.rs"));
        assert!(Naming::Kebab.accepts("file-stats.ts"));
        assert!(!Naming::Kebab.accepts("-x.ts"));
        assert!(Naming::Camel.accepts("fileStats.ts"));
        assert!(!Naming::Camel.accepts("FileStats.ts"));
        assert!(Naming::Pascal.accepts("FileStats.cs"));
        assert!(!Naming::Pascal.accepts("file_stats.cs"));
    }
}
