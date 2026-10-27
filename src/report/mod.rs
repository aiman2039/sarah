//! Aggregation, refactor warnings and rendering (table, markdown, JSON).

mod model;
mod render;
mod warnings;

pub use model::{FileLimits, Report, ReportInput, SortBy, Thresholds};
