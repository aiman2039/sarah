//! Domain units (rust.md: newtypes), so counts and positions can't be mixed up.

mod counts;
mod position;

pub use counts::{Bytes, Cognitive, Cyclomatic, Depth, Lines, Params, Tokens};
pub use position::LineNo;
