//! Function and method spans via tree-sitter tags queries, plus control-flow nesting depth.

mod extract;
mod lang;
mod nesting;

pub use extract::FnStats;
