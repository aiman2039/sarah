use derive_more::Display;
use serde::Deserialize;

/// Common compound suffixes before the final file extension (`App.test.tsx`, `types.d.ts`).
const COMPOUND_SUFFIXES: &[&str] = &[
    ".d",
    ".test",
    ".spec",
    ".stories",
    ".module",
    ".config",
    ".gen",
    ".generated",
];

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
    /// Checks a file name: strips the final extension and known compound suffixes, then
    /// checks every `.`-separated stem part so `app.BadName.ts` cannot slip past.
    pub fn accepts(self, name: &str) -> bool {
        if name.starts_with('.') {
            return true;
        }
        let stem = Self::file_stem(name);
        !stem.is_empty() && stem.split('.').all(|part| self.accepts_part(part))
    }

    /// Checks a folder name: checks every `.`-separated part without stripping an extension.
    pub fn accepts_dir(self, name: &str) -> bool {
        if name.starts_with('.') {
            return true;
        }
        !name.is_empty() && name.split('.').all(|part| self.accepts_part(part))
    }

    fn file_stem(name: &str) -> &str {
        let mut base = name.rsplit_once('.').map_or(name, |(stem, _)| stem);
        while let Some(stripped) = COMPOUND_SUFFIXES
            .iter()
            .find_map(|suf| base.strip_suffix(suf))
        {
            if stripped.is_empty() {
                break;
            }
            base = stripped;
        }
        base
    }

    fn accepts_part(self, stem: &str) -> bool {
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
    fn conventions_and_multi_dot_stems() {
        assert!(Naming::Snake.accepts("file_stats.rs"));
        assert!(Naming::Snake.accepts("__init__.py"));
        assert!(Naming::Snake.accepts(".gitignore"));
        assert!(Naming::Snake.accepts("app.user_profile.ts"));
        assert!(!Naming::Snake.accepts("FileStats.rs"));
        assert!(!Naming::Snake.accepts("app.BadName.ts"));
        assert!(!Naming::Snake.accepts("file-stats.rs"));
        assert!(!Naming::Snake.accepts_dir("v1.BadDir"));
        assert!(Naming::Kebab.accepts("file-stats.ts"));
        assert!(!Naming::Kebab.accepts("-x.ts"));
        assert!(Naming::Camel.accepts("fileStats.ts"));
        assert!(!Naming::Camel.accepts("FileStats.ts"));
        assert!(Naming::Pascal.accepts("FileStats.cs"));
        assert!(Naming::Pascal.accepts("App.test.tsx"));
        assert!(!Naming::Pascal.accepts("App.bad_part.tsx"));
        assert!(!Naming::Pascal.accepts("file_stats.cs"));
    }
}
