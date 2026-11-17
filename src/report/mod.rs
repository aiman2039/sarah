//! Aggregation, refactor warnings and baseline/changed-file filters.

mod allow;
mod baseline;
mod changed;
mod diff;
mod model;
mod single;
mod warning;
mod warnings;

pub use baseline::Baseline;
pub use diff::DiffSummary;
pub use model::{Report, ReportInput, SortBy, Totals};
pub use single::SingleFileInput;
pub use warning::{Warning, WarningKind};
