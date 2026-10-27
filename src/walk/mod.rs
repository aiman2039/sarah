//! Repository walk: gitignore-aware, parallel, symlink-safe file discovery.

mod classify;
mod overrides;
mod skipped;
mod walker;

pub use skipped::Skipped;
pub use walker::{FileEntry, Walk, WalkFilters, WalkOptions};
