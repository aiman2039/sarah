use tokei::LanguageType;

use crate::functions::InlineTestHelpers;
use crate::units::{Chars, LineNo, LineSpan};

/// Code line lengths of one file in characters (comment-only and blank lines excluded).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LineLengths {
    /// The longest code line; `None` for a file without code lines.
    pub longest: Option<Chars>,
    /// Lines over the path's `max_line_length`; `None` without a limit or a longer line.
    pub over: Option<LongLines>,
}

/// Lines over a length limit in one file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LongLines {
    pub count: usize,
    pub first: LineNo,
    pub longest: Chars,
}

/// One file's text and the rules for measuring its line lengths.
pub struct LengthInput<'a> {
    /// Decides which lines are comment-only.
    pub lang: Option<LanguageType>,
    pub text: &'a str,
    /// Lines that do not count (stripped inline tests).
    pub skip: &'a [LineSpan],
    /// The `max_line_length` that `over` reports against.
    pub max: Option<Chars>,
}

/// Comment syntax of one language and whether a block comment is still open.
pub(super) struct CommentScanner {
    line: &'static [&'static str],
    blocks: Vec<(&'static str, &'static str)>,
    open: Option<&'static str>,
}

impl LineLengths {
    /// Lengths in characters (not bytes), in one pass over the text.
    pub fn measure(input: LengthInput<'_>) -> Self {
        let LengthInput {
            lang,
            text,
            skip,
            max,
        } = input;
        let mut scanner = CommentScanner::new(lang);
        let mut out = Self::default();
        for (idx, line) in text.lines().enumerate() {
            let line_no = LineNo(1).below(idx);
            if scanner.is_blank_or_comment(line) || InlineTestHelpers::in_spans(line_no, skip) {
                continue;
            }
            out.add(line_no, Chars(line.chars().count()), max);
        }
        out
    }

    fn add(&mut self, line: LineNo, len: Chars, max: Option<Chars>) {
        self.longest = self.longest.max(Some(len));
        if max.is_none_or(|max| len <= max) {
            return;
        }
        let long = self.over.get_or_insert(LongLines {
            count: 0,
            first: line,
            longest: len,
        });
        long.count = long.count.saturating_add(1);
        long.longest = long.longest.max(len);
    }
}

impl CommentScanner {
    pub(super) fn new(lang: Option<LanguageType>) -> Self {
        let (line, blocks) = lang.map_or((&[][..], Vec::new()), |l| {
            let blocks = l
                .multi_line_comments()
                .iter()
                .chain(l.nested_comments())
                .chain(l.doc_quotes())
                .copied()
                .collect();
            (l.line_comments(), blocks)
        });
        Self {
            line,
            blocks,
            open: None,
        }
    }

    /// Whether `raw` is blank or holds nothing but comment text. Feed every line in order:
    /// block comments carry over from line to line.
    pub(super) fn is_blank_or_comment(&mut self, raw: &str) -> bool {
        raw.trim().is_empty() || self.is_comment_only(raw)
    }

    /// Whether `raw` holds nothing but comment text (inside or around a block comment).
    fn is_comment_only(&mut self, raw: &str) -> bool {
        let trimmed = raw.trim();
        if let Some(end) = self.open {
            let Some((_, rest)) = trimmed.split_once(end) else {
                return true;
            };
            self.open = None;
            return self.rest_is_comment(rest);
        }
        if self.line.iter().any(|lead| trimmed.starts_with(lead)) {
            return true;
        }
        let Some((start, end)) = self
            .blocks
            .iter()
            .find(|(start, _)| trimmed.starts_with(start))
            .copied()
        else {
            return false;
        };
        let after = trimmed.get(start.len()..).unwrap_or_default();
        if let Some((_, rest)) = after.split_once(end) {
            return self.rest_is_comment(rest);
        }
        self.open = Some(end);
        true
    }

    /// Text after a closed block comment: empty, another comment, or code.
    fn rest_is_comment(&mut self, rest: &str) -> bool {
        let rest = rest.trim();
        rest.is_empty() || self.is_comment_only(rest)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn measure(lang: LanguageType, text: &str, max: usize) -> LineLengths {
        LineLengths::measure(LengthInput {
            lang: Some(lang),
            text,
            skip: &[],
            max: Some(Chars(max)),
        })
    }

    #[test]
    fn skips_comments_and_blanks_and_counts_chars() {
        // Code line lengths: L2 = 10 (chars, not bytes), L6 = 18, L7 = 16; the rest are comments.
        let text = "// a long comment line\nlet é = 1;\n\n/* block\n still */\n/* x */ let a = 2;\n#[derive(Debug)]\n";
        let all = measure(LanguageType::Rust, text, 0);
        assert_eq!(all.longest, Some(Chars(18)));
        assert_eq!(
            all.over,
            Some(LongLines {
                count: 3,
                first: LineNo(2),
                longest: Chars(18),
            })
        );
        let over_ten = measure(LanguageType::Rust, text, 10).over.unwrap();
        assert_eq!((over_ten.count, over_ten.first), (2, LineNo(6)));
        let py = measure(
            LanguageType::Python,
            "# c\nx = 1\n\"\"\"doc\nstring\"\"\"\n",
            0,
        );
        assert_eq!(py.over.map(|o| (o.count, o.first)), Some((1, LineNo(2))));
        assert_eq!(py.longest, Some(Chars(5)));
    }

    #[test]
    fn over_reports_count_first_and_longest() {
        let text = "ab\nabcdef\nabcd\nabcdefgh\n";
        let ll = measure(LanguageType::Rust, text, 3);
        assert_eq!(
            ll.over,
            Some(LongLines {
                count: 3,
                first: LineNo(2),
                longest: Chars(8),
            })
        );
        assert_eq!(measure(LanguageType::Rust, text, 8).over, None);
        assert_eq!(ll.longest, Some(Chars(8)));
    }
}
