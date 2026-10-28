use derive_more::Display;
use serde::Deserialize;
use serde::de::value::{Error as ValueError, StrDeserializer};

use crate::report::WarningKind;
use crate::units::LineNo;

const MARKER: &str = "sarah-allow";
const FILE_SUFFIX: &str = "-file";
/// Characters that open a line comment in the supported languages (`//`, `#`, `--`, `/*`, `*`, `<!--`, `;`, `%`).
const COMMENT_LEADERS: &[char] = &['/', '*', '#', '-', '!', '<', ';', '%'];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Display)]
pub enum Scope {
    #[display("function")]
    Function,
    #[display("file")]
    File,
}

/// One `sarah-allow(kinds): reason` or `sarah-allow-file(kinds): reason` comment.
#[derive(Debug, PartialEq, Eq)]
pub struct Directive {
    pub line: LineNo,
    pub scope: Scope,
    pub kinds: Vec<WarningKind>,
}

/// A malformed or misplaced directive, reported as a `bad_allow` warning.
#[derive(Debug, PartialEq, Eq)]
pub struct AllowProblem {
    pub line: LineNo,
    pub message: String,
}

impl Scope {
    /// What a directive of this scope may allow; also the default for an omitted list.
    pub(super) fn allowed_kinds(self) -> &'static [WarningKind] {
        match self {
            Self::Function => &[
                WarningKind::LargeFunction,
                WarningKind::DeepFunction,
                WarningKind::TooManyParams,
                WarningKind::ComplexFunction,
            ],
            Self::File => &[
                WarningKind::LargeFile,
                WarningKind::LargeFunction,
                WarningKind::DeepFunction,
                WarningKind::TooManyParams,
                WarningKind::ComplexFunction,
            ],
        }
    }
}

pub(super) struct DirectiveHelpers;

impl DirectiveHelpers {
    /// `None` when the line holds no directive. Only comment lines count, so the marker
    /// inside strings or after code is ignored.
    pub(super) fn parse(line: LineNo, text: &str) -> Option<Result<Directive, AllowProblem>> {
        let rest = text
            .trim_start()
            .trim_start_matches(COMMENT_LEADERS)
            .trim_start()
            .strip_prefix(MARKER)?;
        let (scope, rest) = match rest.strip_prefix(FILE_SUFFIX) {
            Some(rest) => (Scope::File, rest),
            None => (Scope::Function, rest),
        };
        if rest.starts_with(|c: char| c.is_alphanumeric() || c == '_' || c == '-') {
            return None;
        }
        let problem = |message: String| AllowProblem { line, message };
        Some(
            Self::parse_body(scope, rest)
                .map(|kinds| Directive { line, scope, kinds })
                .map_err(problem),
        )
    }

    /// `(kinds): reason`, `: reason`; the reason may be empty but the `:` is required.
    fn parse_body(scope: Scope, rest: &str) -> Result<Vec<WarningKind>, String> {
        let (list, rest) = match rest.trim_start().strip_prefix('(') {
            Some(inner) => {
                let (list, after) = inner
                    .split_once(')')
                    .ok_or_else(|| "missing `)`".to_string())?;
                (Some(list), after)
            }
            None => (None, rest),
        };
        if !rest.trim_start().starts_with(':') {
            return Err(
                "missing `: <reason>` (the reason may be empty, the `:` is required)".to_string(),
            );
        }
        Self::parse_kinds(scope, list)
    }

    fn parse_kinds(scope: Scope, list: Option<&str>) -> Result<Vec<WarningKind>, String> {
        let names: Vec<&str> = list
            .unwrap_or_default()
            .split(',')
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .collect();
        if names.is_empty() {
            return Ok(scope.allowed_kinds().to_vec());
        }
        names
            .into_iter()
            .map(|name| Self::parse_kind(scope, name))
            .collect()
    }

    fn parse_kind(scope: Scope, name: &str) -> Result<WarningKind, String> {
        let kind = WarningKind::deserialize(StrDeserializer::<ValueError>::new(name))
            .map_err(|_| format!("unknown warning kind `{name}`"))?;
        if !scope.allowed_kinds().contains(&kind) {
            return Err(format!("`{name}` cannot be allowed by a {scope} comment"));
        }
        Ok(kind)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(text: &str) -> Option<Result<Directive, AllowProblem>> {
        DirectiveHelpers::parse(LineNo(3), text)
    }

    fn kinds(text: &str) -> Vec<WarningKind> {
        parse(text).unwrap().unwrap().kinds
    }

    fn problem(text: &str) -> String {
        parse(text).unwrap().unwrap_err().message
    }

    #[test]
    fn parses_kinds_and_scope() {
        let d = parse("    // sarah-allow(large_function, deep_function): state machine")
            .unwrap()
            .unwrap();
        assert_eq!(d.line, LineNo(3));
        assert_eq!(d.scope, Scope::Function);
        assert_eq!(
            d.kinds,
            vec![WarningKind::LargeFunction, WarningKind::DeepFunction]
        );
        let file = parse("# sarah-allow-file(large_file): generated")
            .unwrap()
            .unwrap();
        assert_eq!(file.scope, Scope::File);
        assert_eq!(file.kinds, vec![WarningKind::LargeFile]);
    }

    #[test]
    fn empty_reason_and_list_allow_every_kind_of_the_scope() {
        assert_eq!(kinds("// sarah-allow:"), Scope::Function.allowed_kinds());
        assert_eq!(kinds("-- sarah-allow():"), Scope::Function.allowed_kinds());
        assert_eq!(
            kinds("/* sarah-allow-file: */"),
            Scope::File.allowed_kinds()
        );
        assert_eq!(
            kinds(" * sarah-allow(deep_function) : x"),
            [WarningKind::DeepFunction]
        );
    }

    #[test]
    fn rejects_bad_directives() {
        assert!(problem("// sarah-allow(large_function)").contains("missing `: <reason>`"));
        assert!(problem("// sarah-allow").contains("missing `: <reason>`"));
        assert!(problem("// sarah-allow(large_function: x").contains("missing `)`"));
        assert!(problem("// sarah-allow(huge): x").contains("unknown warning kind `huge`"));
        assert!(
            problem("// sarah-allow(large_file): x").contains("cannot be allowed by a function")
        );
        assert!(problem("# sarah-allow-file(folder_bad_name): x").contains("by a file"));
    }

    #[test]
    fn ignores_non_directives() {
        assert!(parse("let s = \"// sarah-allow: x\";").is_none());
        assert!(parse("f(); // sarah-allow: x").is_none());
        assert!(parse("// sarah-allowed: x").is_none());
        assert!(parse("// sarah-allow-files: x").is_none());
        assert!(parse("// about sarah-allow: x").is_none());
    }
}
