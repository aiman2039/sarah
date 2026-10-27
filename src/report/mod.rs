//! Aggregation, refactor warnings, baseline/changed-file filters and rendering.

mod allow;
mod baseline;
mod changed;
mod model;
mod render;
mod warnings;

pub use baseline::Baseline;
pub use model::{Report, ReportInput, SortBy, Warning, WarningKind};
pub use render::View;
