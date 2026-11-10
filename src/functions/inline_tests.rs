use std::ops::Range;
use std::path::Path;

use tree_sitter::{Node, Parser};

use super::lang::Lang;
use crate::units::{LineNo, LineSpan};

/// Inline test stripping for languages that keep unit tests inside source files (Rust, Zig).
pub struct InlineTestHelpers;

impl InlineTestHelpers {
    /// Production-only source text plus the 1-based line spans of stripped inline tests.
    /// `None` when the language has no inline tests or the file contains none.
    pub fn strip(path: &Path, source: &str) -> Option<(String, Vec<LineSpan>)> {
        let lang = Lang::from_path(path)?;
        if !matches!(lang, Lang::Rust | Lang::Zig) {
            return None;
        }
        let mut parser = Parser::new();
        parser.set_language(&lang.language()).ok()?;
        let tree = parser.parse(source, None)?;
        let ranges = match lang {
            Lang::Rust => Self::rust_ranges(tree.root_node(), source),
            Lang::Zig => Self::zig_ranges(tree.root_node()),
            _ => Vec::new(),
        };
        if ranges.is_empty() {
            return None;
        }
        let prod = Self::slice_without(source, &ranges);
        let spans = ranges.into_iter().map(|(_, span)| span).collect();
        Some((prod, spans))
    }

    pub fn in_spans(line: LineNo, spans: &[LineSpan]) -> bool {
        spans.iter().any(|s| line >= s.start && line <= s.end)
    }

    fn rust_ranges(root: Node, source: &str) -> Vec<(Range<usize>, LineSpan)> {
        let mut out = Vec::new();
        let mut attr_start: Option<(usize, LineNo)> = None;
        let mut test_marked = false;
        let mut cursor = root.walk();
        for node in root.named_children(&mut cursor) {
            if node.kind() == "attribute_item" {
                attr_start.get_or_insert((
                    node.start_byte(),
                    LineNo(node.start_position().row.saturating_add(1)),
                ));
                if let Some(txt) = source.get(node.byte_range())
                    && Self::is_rust_test_attr(txt)
                {
                    test_marked = true;
                }
                continue;
            }
            if test_marked
                && matches!(node.kind(), "mod_item" | "function_item")
                && let Some((byte_start, line_start)) = attr_start
            {
                let end_line = LineNo(node.end_position().row.saturating_add(1));
                out.push((
                    byte_start..node.end_byte(),
                    LineSpan {
                        start: line_start,
                        end: end_line,
                    },
                ));
            }
            attr_start = None;
            test_marked = false;
        }
        out
    }

    fn is_rust_test_attr(txt: &str) -> bool {
        txt.contains("cfg(test)") || txt.contains("#[test]") || txt.contains("::test]")
    }

    fn zig_ranges(root: Node) -> Vec<(Range<usize>, LineSpan)> {
        let mut out = Vec::new();
        let mut cursor = root.walk();
        for node in root.named_children(&mut cursor) {
            if node.kind() == "test_declaration" {
                let span = LineSpan {
                    start: LineNo(node.start_position().row.saturating_add(1)),
                    end: LineNo(node.end_position().row.saturating_add(1)),
                };
                out.push((node.byte_range(), span));
            }
        }
        out
    }

    fn slice_without(source: &str, ranges: &[(Range<usize>, LineSpan)]) -> String {
        let mut out = String::with_capacity(source.len());
        let mut cursor = 0usize;
        for (range, _) in ranges {
            if let Some(chunk) = source.get(cursor..range.start) {
                out.push_str(chunk);
            }
            cursor = cursor.max(range.end);
        }
        if let Some(tail) = source.get(cursor..) {
            out.push_str(tail);
        }
        out
    }
}
