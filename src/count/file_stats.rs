use std::path::{Path, PathBuf};

use serde::Serialize;
use tokei::{Config, LanguageType};

use super::comments::{CommentHelpers, CommentInfo};
use super::generated::GeneratedHelpers;
use super::tokenizer::Tokenizer;
use crate::allow::Allows;
use crate::config::{AnalysisRules, PatternHit};
use crate::functions::{AstHelpers, AstStats, FnStats, ImportRef, InlineTestHelpers, TypeStats};
use crate::lines::{InlineTestLines, LengthInput, LineBreakdown, LineLengths};
use crate::units::Items;
use crate::units::{Bytes, LineNo, LineSpan, Lines, Tokens};
use crate::walk::FileEntry;

#[derive(Clone, Debug, Default, Serialize)]
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
    pub module_decls: Vec<String>,
    #[serde(skip)]
    pub comment_info: CommentInfo,
    /// Normalized non-trivial code lines for duplicate detection: `(1-based line, trimmed)`.
    #[serde(skip)]
    pub norm_lines: Vec<(usize, String)>,
    /// File-wide `sarah-allow-file` kinds, tracked directives and malformed directives.
    #[serde(skip)]
    pub allows: Allows,
    /// Lines matching the file's `forbid_patterns` (capped per pattern).
    #[serde(skip)]
    pub pattern_hits: Vec<PatternHit>,
    /// Inline unit tests, for `require_test` and the folder test ratio; zero unless
    /// `AnalysisRules::find_inline_tests` is on.
    #[serde(skip)]
    pub inline_tests: InlineTestLines,
    /// Public API items; `None` when the language has no visibility rule.
    #[serde(skip)]
    pub public_items: Option<Items>,
    /// Longest code line and the lines over `max_line_length`.
    #[serde(skip)]
    pub line_lengths: LineLengths,
}

/// Switches that hold for every file of a run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AnalysisRun {
    pub tokenizer: Tokenizer,
    pub with_functions: bool,
    /// Leave out files with a generated-code header.
    pub skip_generated: bool,
}

pub struct AnalyzeInput<'a> {
    pub entry: FileEntry,
    pub run: AnalysisRun,
    pub rules: AnalysisRules<'a>,
}

pub struct SourceInput<'a> {
    pub path: PathBuf,
    pub text: &'a str,
    pub run: AnalysisRun,
    pub rules: AnalysisRules<'a>,
}

/// One non-generated file and its inline tests, ready to measure.
struct MeasureInput<'a> {
    source: SourceInput<'a>,
    /// `source.text` without stripped tests (`exclude_inline_tests`).
    measured: &'a str,
    /// Every inline test found, stripped or not.
    test_spans: &'a [LineSpan],
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
    pub fn analyze(input: AnalyzeInput<'_>) -> Analysis {
        let AnalyzeInput {
            entry: FileEntry { real, display },
            run,
            rules,
        } = input;
        let Ok(text) = std::fs::read_to_string(&real) else {
            return Analysis::NotText;
        };
        Self::analyze_source(SourceInput {
            path: display,
            text: &text,
            run,
            rules,
        })
    }

    /// Inline tests are parsed only when `rules.find_inline_tests` is on, and the text is
    /// copied without them only when `rules.exclude_inline_tests` is on.
    pub fn analyze_source(source: SourceInput<'_>) -> Analysis {
        if source.run.skip_generated && GeneratedHelpers::is_generated(source.text) {
            return Analysis::Generated;
        }
        let tests = Self::find_tests(&source);
        let stripped = (source.rules.exclude_inline_tests && !tests.is_empty())
            .then(|| InlineTestHelpers::without(source.text, &tests));
        let test_spans: Vec<LineSpan> = tests.into_iter().map(|(_, span)| span).collect();
        Analysis::Counted(Box::new(Self::measure(MeasureInput {
            measured: stripped.as_deref().unwrap_or(source.text),
            source,
            test_spans: &test_spans,
        })))
    }

    fn find_tests(source: &SourceInput<'_>) -> Vec<(std::ops::Range<usize>, LineSpan)> {
        if !source.rules.find_inline_tests || !source.text.contains("test") {
            return Vec::new();
        }
        InlineTestHelpers::find(&source.path, source.text)
    }

    /// Every measurement of one non-generated file.
    fn measure(input: MeasureInput<'_>) -> Self {
        let MeasureInput {
            source,
            measured,
            test_spans,
        } = input;
        let SourceInput {
            path,
            text,
            run,
            rules,
        } = source;
        let lang = LanguageType::from_path(&path, &Config::default());
        let skip = rules.stripped(test_spans);
        let breakdown = LineBreakdown::of(lang, measured);
        let (ast, allows) = Self::ast_and_allows((&path, text), run, skip);
        Self {
            language: lang.map_or("Other".to_string(), |l| l.name().to_string()),
            bytes: Bytes(measured.len()),
            lines: Lines(measured.lines().count()),
            code: breakdown.code,
            comments: breakdown.comments,
            blanks: breakdown.blanks,
            tokens: run.tokenizer.count(measured),
            comment_info: CommentHelpers::scan(&path, measured),
            norm_lines: Self::normalized_lines(&path, text, skip),
            pattern_hits: (rules.forbid_patterns).map_or_else(Vec::new, |p| p.scan(text, skip)),
            inline_tests: InlineTestLines {
                code: InlineTestLines::code_in(lang, text, test_spans),
                stripped: rules.exclude_inline_tests,
            },
            line_lengths: LineLengths::measure(LengthInput {
                lang,
                text,
                skip,
                max: rules.max_line_length,
            }),
            path,
            ..Self::from_ast(ast, allows)
        }
    }

    /// The fields read from the syntax tree, plus `allows`; the rest stay default.
    fn from_ast(ast: Option<AstStats>, allows: Allows) -> Self {
        let Some(ast) = ast else {
            return Self {
                allows,
                ..Self::default()
            };
        };
        Self {
            functions: Some(ast.functions),
            types: ast.types,
            imports: ast.imports,
            module_decls: ast.module_decls,
            public_items: ast.public_items,
            allows,
            ..Self::default()
        }
    }

    /// The syntax tree (`None` without function extraction) and the `sarah-allow` comments,
    /// both without the stripped tests in `skip`.
    fn ast_and_allows(
        src: (&Path, &str),
        run: AnalysisRun,
        skip: &[LineSpan],
    ) -> (Option<AstStats>, Allows) {
        let (path, text) = src;
        let mut ast = run
            .with_functions
            .then(|| AstStats::extract(path, text, run.tokenizer))
            .flatten();
        Self::drop_test_ast(ast.as_mut(), skip);
        let mut allows = if AstHelpers::is_code_file(path) {
            Allows::scan(text, ast.as_mut())
        } else {
            Allows::default()
        };
        Self::drop_test_allows(&mut allows, skip);
        (ast, allows)
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
                if InlineTestHelpers::in_spans(LineNo(line_no), spans) {
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
