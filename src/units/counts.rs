use derive_more::{Display, FromStr};
use serde::Serialize;

/// Token count under the selected encoding.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Display, FromStr, Serialize,
)]
#[serde(transparent)]
pub struct Tokens(pub usize);

/// Number of lines (a count, not a position; see `LineNo`).
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Display, FromStr, Serialize,
)]
#[serde(transparent)]
pub struct Lines(pub usize);

/// Size in bytes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Display, Serialize)]
#[serde(transparent)]
pub struct Bytes(pub usize);

impl Tokens {
    pub fn saturating_add(self, rhs: Self) -> Self {
        Self(self.0.saturating_add(rhs.0))
    }
}

impl Lines {
    pub fn saturating_add(self, rhs: Self) -> Self {
        Self(self.0.saturating_add(rhs.0))
    }
}

impl Bytes {
    pub fn saturating_add(self, rhs: Self) -> Self {
        Self(self.0.saturating_add(rhs.0))
    }
}
