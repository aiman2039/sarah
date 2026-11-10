use std::path::Path;

use crate::functions::AstHelpers;
use crate::units::LineNo;

const TODO_MARKERS: [&str; 4] = ["TODO", "FIXME", "HACK", "XXX"];
const LINE_COMMENT_PREFIXES: [&str; 4] = ["//", "#", "--", "/*"];

#[derive(Clone, Debug, Default)]
pub struct CommentInfo {
    pub todos: Vec<(LineNo, &'static str)>,
    pub lines: Vec<(LineNo, String)>,
}

/// Extracts comment lines and task markers in source files.
pub struct CommentHelpers;

impl CommentHelpers {
    pub fn scan(path: &Path, text: &str) -> CommentInfo {
        if !AstHelpers::is_code_file(path) {
            return CommentInfo::default();
        }
        let mut info = CommentInfo::default();
        let mut in_block = false;
        for (idx, raw) in text.lines().enumerate() {
            let line_no = LineNo(1).below(idx);
            let Some(body) = Self::comment_part(raw, &mut in_block) else {
                continue;
            };
            let cleaned = body.trim_start_matches(['/', '!']).trim();
            if let Some(marker) = Self::find_todo(cleaned) {
                info.todos.push((line_no, marker));
            }
            info.lines.push((line_no, cleaned.to_string()));
        }
        info
    }

    fn comment_part<'a>(raw: &'a str, in_block: &mut bool) -> Option<&'a str> {
        let trimmed = raw.trim();
        if *in_block {
            if let Some((before, _)) = trimmed.split_once("*/") {
                *in_block = false;
                return Some(before);
            }
            return Some(trimmed.trim_start_matches('*'));
        }
        if let Some((_, after)) = trimmed.split_once("/*") {
            if let Some((inside, _)) = after.split_once("*/") {
                return Some(inside);
            }
            *in_block = true;
            return Some(after);
        }
        for prefix in LINE_COMMENT_PREFIXES {
            if let Some(rest) = trimmed.strip_prefix(prefix) {
                return Some(rest);
            }
        }
        None
    }

    /// Matches `TODO`, `TODO:`, `TODO(owner):` at the start of a comment (not prose mentions).
    fn find_todo(cleaned: &str) -> Option<&'static str> {
        for &marker in &TODO_MARKERS {
            if let Some(rest) = cleaned.strip_prefix(marker)
                && (rest.is_empty() || rest.starts_with([':', '(', ' ', '\t', '-']))
            {
                return Some(marker);
            }
        }
        None
    }
}
