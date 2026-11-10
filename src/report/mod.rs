//! Aggregation, refactor warnings, baseline/changed-file filters and rendering.

mod allow;
mod baseline;
mod changed;
mod diff;
mod model;
mod render;
mod warning;
mod warnings;

pub use baseline::Baseline;
pub use diff::DiffSummary;
pub use model::{Report, ReportInput, SortBy};
pub use render::View;
pub use warning::{Warning, WarningKind};
