//! Folder structure: per-folder totals and the `[[folder]]` rules checked against them.

mod check;
mod entries;
mod naming;
mod preset;
mod rules;
mod tree;

pub use preset::Preset;
pub use rules::{FolderLimitKey, FolderPatch, FolderRule, FolderRules};
pub use tree::{DirNode, DirStats, DirTree};
