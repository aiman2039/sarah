use std::path::PathBuf;

use serde::Serialize;
use tokei::{Config, LanguageType};

use super::generated::GeneratedHelpers;
use super::tokenizer::Tokenizer;
use crate::allow::Allows;
use crate::functions::FnStats;
use crate::units::{Bytes, Lines, Tokens};
use crate::walk::FileEntry;

#[derive(Debug, Serialize)]
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
    /// File-wide `sarah-allow-file` kinds and malformed directives.
    #[serde(skip)]
    pub allows: Allows,
}

pub struct AnalyzeInput {
    pub entry: FileEntry,
    pub tokenizer: Tokenizer,
    pub with_functions: bool,
    /// Leave out files with a generated-code header.
    pub skip_generated: bool,
}

/// What reading one walked file gave.
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
        } = input;
        let Ok(text) = std::fs::read_to_string(&real) else {
            return Analysis::NotText;
        };
        if skip_generated && GeneratedHelpers::is_generated(&text) {
            return Analysis::Generated;
        }
        let lang = LanguageType::from_path(&real, &Config::default());
        let LineBreakdown {
            code,
            comments,
            blanks,
        } = LineBreakdown::of(lang, &text);
        let mut functions = with_functions
            .then(|| FnStats::extract(&real, &text, tokenizer))
            .flatten();
        let allows = Allows::scan(&text, functions.as_deref_mut());
        Analysis::Counted(Box::new(Self {
            language: lang.map_or("Other".to_string(), |l| l.name().to_string()),
            bytes: Bytes(text.len()),
            lines: Lines(text.lines().count()),
            code,
            comments,
            blanks,
            tokens: tokenizer.count(&text),
            functions,
            allows,
            path: display,
        }))
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
