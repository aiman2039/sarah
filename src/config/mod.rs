//! `sarah.toml`: walk filters, per-glob size limits and folder-structure rules.

mod file;
mod glob;
mod limits;

pub use file::Config;
pub use glob::{DirPattern, NamePatterns};
pub use limits::{LimitKey, LimitPatch, LimitRules, Limits};
