//! Renderers: table and markdown for people, GitHub annotations and SARIF for CI.
//! JSON is `Report::to_json`.

mod common;
mod github;
mod markdown;
mod sarif;
mod table;

/// What the human-readable renderers show.
#[derive(Clone, Copy, Debug)]
pub struct View {
    /// Rows in the file, function and folder tables.
    pub top: usize,
    /// Show the folder table.
    pub dirs: bool,
}
