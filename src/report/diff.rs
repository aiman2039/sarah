use serde::Serialize;

use super::model::Report;

/// Deltas against a git revision (`--diff REV`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct DiffSummary {
    pub rev: String,
    pub files_delta: isize,
    pub lines_delta: isize,
    pub code_delta: isize,
    pub tokens_delta: isize,
    pub warnings_delta: isize,
}

impl DiffSummary {
    pub fn between(rev: String, base: &Report, current: &Report) -> Self {
        Self {
            rev,
            files_delta: Self::delta(current.totals.files, base.totals.files),
            lines_delta: Self::delta(current.totals.lines.0, base.totals.lines.0),
            code_delta: Self::delta(current.totals.code.0, base.totals.code.0),
            tokens_delta: Self::delta(current.totals.tokens.0, base.totals.tokens.0),
            warnings_delta: Self::delta(current.warnings.len(), base.warnings.len()),
        }
    }

    pub(super) fn table_line(&self) -> String {
        format!(
            "Diff vs {}: {} files, {} lines ({} code), {} tokens, {} warnings\n",
            self.rev,
            Self::signed(self.files_delta),
            Self::signed(self.lines_delta),
            Self::signed(self.code_delta),
            Self::signed(self.tokens_delta),
            Self::signed(self.warnings_delta),
        )
    }

    pub(super) fn markdown_line(&self) -> String {
        format!(
            "Diff vs `{}`: **{} files**, {} lines ({} code), **{} tokens**, **{} warnings**\n\n",
            self.rev,
            Self::signed(self.files_delta),
            Self::signed(self.lines_delta),
            Self::signed(self.code_delta),
            Self::signed(self.tokens_delta),
            Self::signed(self.warnings_delta),
        )
    }

    fn delta(now: usize, then: usize) -> isize {
        if now >= then {
            isize::try_from(now.saturating_sub(then)).unwrap_or(isize::MAX)
        } else {
            let mag = isize::try_from(then.saturating_sub(now)).unwrap_or(isize::MAX);
            0isize.saturating_sub(mag)
        }
    }

    fn signed(n: isize) -> String {
        if n >= 0 {
            format!("+{n}")
        } else {
            n.to_string()
        }
    }
}
