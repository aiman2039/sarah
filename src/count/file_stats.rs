use std::path::{Path, PathBuf};

use serde::Serialize;
use tokei::{Config, LanguageType};

use super::comments::{CommentHelpers, CommentInfo};
use super::generated::GeneratedHelpers;
use super::tokenizer::Tokenizer;
use crate::allow::Allows;
use crate::functions::{AstHelpers, AstStats, FnStats, ImportRef, InlineTestHelpers, TypeStats};
use crate::units::{Bytes, LineSpan, Lines, Tokens};
use crate::walk::FileEntry;

#[derive(Clone, Debug, Serialize)]
pub struct FileStats {
    pub path: PathBuf,
    pub language: String,
    pub bytes: Bytes,
    pub lines: Lines,
    pub code: Lines,
    pub comments: Lines,
    pub blanks: Lines,
    pub tokens: Tokens,
    /// `None` when function extraction isn't supported for this language or was disabled.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub functions: Option<Vec<FnStats>>,
    #[serde(skip)]
    pub types: Vec<TypeStats>,
    #[serde(skip)]
    pub imports: Vec<ImportRef>,
    #[serde(skip)]
    pub comment_info: CommentInfo,
    /// Normalized non-trivial code lines for duplicate detection: `(1-based line, trimmed)`.
    #[serde(skip)]
    pub norm_lines: Vec<(usize, String)>,
    /// File-wide `sarah-allow-file` kinds, tracked directives and malformed directives.
    #[serde(skip)]
    pub allows: Allows,
}

pub struct AnalyzeInput {
    pub entry: FileEntry,
    pub tokenizer: Tokenizer,
    pub with_functions: bool,
    /// Leave out files with a generated-code header.
    pub skip_generated: bool,
    /// Strip inline unit tests (`#[cfg(test)]`, Zig `test`) before counting.
    pub exclude_inline_tests: bool,
}

pub struct SourceInput<'a> {
    pub path: PathBuf,
    pub text: &'a str,
    pub tokenizer: Tokenizer,
    pub with_functions: bool,
    pub skip_generated: bool,
    pub exclude_inline_tests: bool,
}

/// What reading one walked file gave.
#[derive(Clone)]
pub enum Analysis {
    Counted(Box<FileStats>),
    /// Has a generated-code header; counted in `Skipped::generated`.
    Generated,
    /// Not valid UTF-8 (binaries etc.).
    NotText,
}

impl FileStats {
    pub fn analyze(input: AnalyzeInput) -> Analysis {
        let AnalyzeInput {
            entry: FileEntry { real, display },
            tokenizer,
            with_functions,
            skip_generated,
            exclude_inline_tests,
        } = input;
        let Ok(text) = std::fs::read_to_string(&real) else {
            return Analysis::NotText;
        };
        Self::analyze_source(SourceInput {
            path: display,
            text: &text,
            tokenizer,
            with_functions,
            skip_generated,
            exclude_inline_tests,
        })
    }

    pub fn analyze_source(input: SourceInput<'_>) -> Analysis {
        let SourceInput {
            path,
            text,
            tokenizer,
            with_functions,
            skip_generated,
            exclude_inline_tests,
        } = input;
        if skip_generated && GeneratedHelpers::is_generated(text) {
            return Analysis::Generated;
        }
        let stripped = exclude_inline_tests
            .then(|| InlineTestHelpers::strip(&path, text))
            .flatten();
        let measured = stripped.as_ref().map_or(text, |(prod, _)| prod.as_str());
        let test_spans = stripped
            .as_ref()
            .map_or(&[][..], |(_, spans)| spans.as_slice());

        let lang = LanguageType::from_path(&path, &Config::default());
        let LineBreakdown {
            code,
            comments,
            blanks,
        } = LineBreakdown::of(lang, measured);
        let (functions, types, imports, allows) =
            Self::ast_and_allows((&path, text), (tokenizer, with_functions), test_spans);
        Analysis::Counted(Box::new(Self {
            language: lang.map_or("Other".to_string(), |l| l.name().to_string()),
            bytes: Bytes(measured.len()),
            lines: Lines(measured.lines().count()),
            code,
            comments,
            blanks,
            tokens: tokenizer.count(measured),
            functions,
            types,
            imports,
            comment_info: CommentHelpers::scan(&path, measured),
            norm_lines: Self::normalized_lines(&path, text, test_spans),
            allows,
            path,
        }))
    }

    fn ast_and_allows(
        src: (&Path, &str),
        opts: (Tokenizer, bool),
        test_spans: &[LineSpan],
    ) -> (Option<Vec<FnStats>>, Vec<TypeStats>, Vec<ImportRef>, Allows) {
        let (path, text) = src;
        let (tokenizer, with_functions) = opts;
        let mut ast = with_functions
            .then(|| AstStats::extract(path, text, tokenizer))
            .flatten();
        Self::drop_test_ast(ast.as_mut(), test_spans);
        let mut allows = if AstHelpers::is_code_file(path) {
            Allows::scan(text, ast.as_mut())
        } else {
            Allows::default()
        };
        Self::drop_test_allows(&mut allows, test_spans);
        match ast {
            Some(ast) => (Some(ast.functions), ast.types, ast.imports, allows),
            None => (None, Vec::new(), Vec::new(), allows),
        }
    }

    fn drop_test_ast(ast: Option<&mut AstStats>, spans: &[LineSpan]) {
        let Some(ast) = ast else { return };
        if spans.is_empty() {
            return;
        }
        ast.functions
            .retain(|f| !InlineTestHelpers::in_spans(f.start_line, spans));
        ast.types
            .retain(|t| !InlineTestHelpers::in_spans(t.start_line, spans));
        ast.imports
            .retain(|i| !InlineTestHelpers::in_spans(i.line, spans));
    }

    fn drop_test_allows(allows: &mut Allows, spans: &[LineSpan]) {
        if spans.is_empty() {
            return;
        }
        allows
            .tracked
            .retain(|t| !InlineTestHelpers::in_spans(t.line, spans));
        allows
            .problems
            .retain(|p| !InlineTestHelpers::in_spans(p.line, spans));
    }

    fn normalized_lines(path: &Path, text: &str, spans: &[LineSpan]) -> Vec<(usize, String)> {
        if !AstHelpers::is_code_file(path) {
            return Vec::new();
        }
        text.lines()
            .enumerate()
            .filter_map(|(idx, raw)| {
                let line_no = idx.saturating_add(1);
                if InlineTestHelpers::in_spans(crate::units::LineNo(line_no), spans) {
                    return None;
                }
                let t = raw.trim();
                let is_comment = ["//", "#", "--", "/*", "*"]
                    .iter()
                    .any(|p| t.starts_with(p));
                (!is_comment && t.len() >= 6).then(|| (line_no, t.to_string()))
            })
            .collect()
    }
}

/// Code/comment/blank split: tokei for known languages, blank-vs-other otherwise.
struct LineBreakdown {
    code: Lines,
    comments: Lines,
    blanks: Lines,
}

impl LineBreakdown {
    fn of(lang: Option<LanguageType>, text: &str) -> Self {
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
