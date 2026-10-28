//! Function and method spans via tree-sitter tags queries, plus per-function metrics:
//! control-flow nesting depth, parameter count and cyclomatic/cognitive complexity.

mod complexity;
mod extract;
mod lang;
mod nesting;
mod params;
mod scope;

pub use extract::FnStats;
