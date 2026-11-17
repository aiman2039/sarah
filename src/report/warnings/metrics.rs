//! Returns per function; types, line length, comment ratio and public items per file.

use tokei::LanguageType;

use crate::config::Limits;
use crate::count::FileStats;
use crate::lines::LongLines;
use crate::report::model::TopFn;
use crate::report::warning::{Warning, WarningKind};
use crate::units::Ratio;

impl Warning {
    pub(super) fn for_many_returns(func: &TopFn, limits: &Limits) -> Option<Self> {
        let max = limits.func.returns.filter(|max| func.returns > *max)?;
        Some(Self::function(
            func,
            WarningKind::TooManyReturns,
            format!(
                "{} return statements (>{max}) — reduce exit points or extract the branches",
                func.returns
            ),
        ))
    }

    pub(super) fn for_file_types(f: &FileStats, limits: &Limits) -> Option<Self> {
        let max = limits.file_types?;
        let count = f.types.len();
        (count > max.0).then(|| {
            Self::file(
                f,
                WarningKind::TooManyTypes,
                format!("{count} types (>{max}) — move types into their own modules"),
            )
        })
    }

    pub(super) fn for_long_lines(f: &FileStats, limits: &Limits) -> Option<Self> {
        let max = limits.line_length?;
        let LongLines {
            count,
            first,
            longest,
        } = f.line_lengths.over?;
        let lines = if count == 1 { "line" } else { "lines" };
        Some(
            Self::file(
                f,
                WarningKind::LongLine,
                format!(
                    "{count} {lines} longer than {max} chars (first at L{first}, longest {longest})"
                ),
            )
            .at_line(first),
        )
    }

    pub(super) fn for_low_comment_ratio(f: &FileStats, limits: &Limits) -> Option<Self> {
        let min = limits.min_comment_ratio?;
        let ratio = Self::comment_ratio(f)?;
        ratio.below(min).then(|| {
            Self::file(
                f,
                WarningKind::LowCommentRatio,
                format!(
                    "comment ratio {}% (<{min}%): {} comment lines for {} code lines — document the intent",
                    ratio.percent(),
                    ratio.part,
                    f.code
                ),
            )
        })
    }

    pub(super) fn for_high_comment_ratio(f: &FileStats, limits: &Limits) -> Option<Self> {
        let max = limits.max_comment_ratio?;
        let ratio = Self::comment_ratio(f)?;
        ratio.above(max).then(|| {
            Self::file(
                f,
                WarningKind::HighCommentRatio,
                format!(
                    "comment ratio {}% (>{max}%): {} comment lines for {} code lines — remove commented-out code and stale comments",
                    ratio.percent(),
                    ratio.part,
                    f.code
                ),
            )
        })
    }

    pub(super) fn for_public_items(f: &FileStats, limits: &Limits) -> Option<Self> {
        let max = limits.public_items?;
        let count = f.public_items?;
        (count > max).then(|| {
            Self::file(
                f,
                WarningKind::TooManyPublicItems,
                format!(
                    "{count} public items (>{max}) — narrow the public API or split the module"
                ),
            )
        })
    }

    /// Comment lines of code + comment lines; `None` for files without code lines or
    /// in languages without comment syntax.
    fn comment_ratio(f: &FileStats) -> Option<Ratio> {
        if f.code.0 == 0 || f.language == "Other" {
            return None;
        }
        let lang = LanguageType::from_name(&f.language)?;
        if lang.line_comments().is_empty() && lang.multi_line_comments().is_empty() {
            return None;
        }
        Some(Ratio {
            part: f.comments.0,
            whole: f.code.0.saturating_add(f.comments.0),
        })
    }
}
