use tokei::LanguageType;

use super::line_length::CommentScanner;
use crate::functions::InlineTestHelpers;
use crate::units::{LineNo, LineSpan, Lines};

/// Inline unit tests (`#[cfg(test)]`, Zig `test`) of one file, measured on the original text.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct InlineTestLines {
    /// Non-blank, non-comment lines inside inline tests.
    pub code: Lines,
    /// `true` when the tests were stripped before counting (`exclude_inline_tests`),
    /// so `FileStats::code` does not include `code`.
    pub stripped: bool,
}

impl InlineTestLines {
    /// Non-blank, non-comment lines of `text` inside `spans`; `lang` decides comment syntax.
    pub fn code_in(lang: Option<LanguageType>, text: &str, spans: &[LineSpan]) -> Lines {
        if spans.is_empty() {
            return Lines(0);
        }
        let mut scanner = CommentScanner::new(lang);
        let count = text
            .lines()
            .enumerate()
            .filter(|(idx, raw)| {
                !scanner.is_blank_or_comment(raw)
                    && InlineTestHelpers::in_spans(LineNo(1).below(*idx), spans)
            })
            .count();
        Lines(count)
    }

    pub fn has_tests(self) -> bool {
        self.code > Lines(0)
    }

    /// Code lines of a file that are not inline tests.
    pub fn non_test_code(self, code: Lines) -> Lines {
        if self.stripped {
            code
        } else {
            Lines(code.0.saturating_sub(self.code.0))
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    #[test]
    fn counts_rust_test_module_lines() {
        let src = "fn a() {}\n\n#[cfg(test)]\nmod tests {\n    // note\n    /* block\n       note */\n\n    #[test]\n    fn t() {}\n}\n";
        let spans: Vec<LineSpan> = InlineTestHelpers::find(Path::new("a.rs"), src)
            .into_iter()
            .map(|(_, span)| span)
            .collect();
        let got = InlineTestLines {
            code: InlineTestLines::code_in(Some(LanguageType::Rust), src, &spans),
            stripped: false,
        };
        assert_eq!(got.code, Lines(5));
        assert_eq!(got.non_test_code(Lines(6)), Lines(1));
        assert!(InlineTestHelpers::find(Path::new("a.py"), "def test(): pass\n").is_empty());
    }
}
