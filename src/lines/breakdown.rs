use tokei::{Config, LanguageType};

use crate::units::Lines;

/// Code/comment/blank split: tokei for known languages, blank-vs-other otherwise.
pub struct LineBreakdown {
    pub code: Lines,
    pub comments: Lines,
    pub blanks: Lines,
}

impl LineBreakdown {
    pub fn of(lang: Option<LanguageType>, text: &str) -> Self {
        let Some(lang) = lang else {
            let blanks = text.lines().filter(|l| l.trim().is_empty()).count();
            return Self {
                code: Lines(text.lines().count().saturating_sub(blanks)),
                comments: Lines(0),
                blanks: Lines(blanks),
            };
        };
        let s = lang.parse_from_str(text, &Config::default()).summarise();
        Self {
            code: Lines(s.code),
            comments: Lines(s.comments),
            blanks: Lines(s.blanks),
        }
    }
}
