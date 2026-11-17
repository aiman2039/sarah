//! `sarah.toml`: walk filters, per-glob size limits and folder-structure rules.

mod file;
mod glob;
mod keys;
mod limits;
mod patterns;
mod test_templates;

pub use file::Config;
pub use glob::{DEFAULT_TEST_GLOBS, DirPattern, GlobHelpers, NamePatterns};
pub use keys::LimitKey;
pub use limits::{AnalysisKey, AnalysisRules, FlagPatch, FnPatch, LimitPatch, LimitRules, Limits};
pub use patterns::PatternHit;
