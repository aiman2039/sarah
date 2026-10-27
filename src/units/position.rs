use derive_more::Display;
use serde::Serialize;

use super::counts::Lines;

/// 1-based line number (a position, not a count; see `Lines`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Display, Serialize)]
#[serde(transparent)]
pub struct LineNo(pub usize);

impl LineNo {
    /// The line `newlines` line breaks below this one.
    pub fn below(self, newlines: usize) -> Self {
        Self(self.0.saturating_add(newlines))
    }

    /// Lines spanned from `self` through `end`, inclusive (at least one).
    pub fn span_to(self, end: Self) -> Lines {
        Lines(end.0.saturating_sub(self.0).saturating_add(1))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn below_and_span_to() {
        let start = LineNo(3);
        assert_eq!(start.below(0), LineNo(3));
        assert_eq!(start.below(4), LineNo(7));
        assert_eq!(start.span_to(LineNo(7)), Lines(5));
        assert_eq!(start.span_to(start), Lines(1));
        assert_eq!(start.span_to(LineNo(1)), Lines(1));
        assert_eq!(LineNo(usize::MAX).below(1), LineNo(usize::MAX));
    }
}
