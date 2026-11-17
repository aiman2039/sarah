use std::str::FromStr;

use derive_more::{Display, Error, FromStr, Into};
use serde::{Deserialize, Serialize};

/// Whole percent in `0..=100`; the checked constructor rejects anything above 100.
#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Display,
    Into,
    Serialize,
    Deserialize,
)]
#[into(u64, usize)]
#[serde(try_from = "u64", into = "u64")]
pub struct Percent(u8);

/// Ratio in percent, e.g. test code lines per 100 non-test code lines. May exceed 100.
#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Display,
    FromStr,
    Into,
    Serialize,
    Deserialize,
)]
#[serde(transparent)]
pub struct RatioPercent(pub usize);

/// `part` of `whole`, compared against a percent without rounding.
#[derive(Clone, Copy, Debug)]
pub struct Ratio {
    pub part: usize,
    pub whole: usize,
}

/// A value that is not a whole percent in `0..=100`.
#[derive(Clone, Debug, PartialEq, Eq, Display, Error)]
pub enum PercentError {
    #[display("{value} is not a percent (0..=100)")]
    OutOfRange { value: u64 },
    #[display("{text:?} is not a whole number (0..=100)")]
    NotANumber { text: String },
}

impl Percent {
    pub const MAX: u8 = 100;

    pub fn new(value: u64) -> Result<Self, PercentError> {
        u8::try_from(value)
            .ok()
            .filter(|v| *v <= Self::MAX)
            .map(Self)
            .ok_or(PercentError::OutOfRange { value })
    }

    pub fn get(self) -> u8 {
        self.0
    }
}

impl TryFrom<u64> for Percent {
    type Error = PercentError;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl Ratio {
    pub fn below(self, pct: impl Into<usize>) -> bool {
        self.scaled() < self.whole.saturating_mul(pct.into())
    }

    pub fn above(self, pct: impl Into<usize>) -> bool {
        self.scaled() > self.whole.saturating_mul(pct.into())
    }

    /// Whole percent, rounded down; 0 when `whole` is 0.
    pub fn percent(self) -> RatioPercent {
        RatioPercent(self.scaled().checked_div(self.whole).unwrap_or(0))
    }

    fn scaled(self) -> usize {
        self.part.saturating_mul(100)
    }
}

impl FromStr for Percent {
    type Err = PercentError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let value = text
            .trim()
            .parse::<u64>()
            .map_err(|_| PercentError::NotANumber {
                text: text.to_string(),
            })?;
        Self::new(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_zero_to_hundred_only() {
        assert_eq!(Percent::new(0).unwrap().get(), 0);
        assert_eq!(Percent::new(100).unwrap().get(), 100);
        assert!(Percent::new(101).is_err());
        assert!("x".parse::<Percent>().is_err());
        assert_eq!("42".parse::<Percent>().unwrap().get(), 42);
        assert!(toml::from_str::<std::collections::HashMap<String, Percent>>("a = 101").is_err());
        let ok: std::collections::HashMap<String, Percent> = toml::from_str("a = 7").unwrap();
        assert_eq!(ok["a"].get(), 7);
    }
}
