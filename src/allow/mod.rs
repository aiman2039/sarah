//! `sarah-allow` comments: per-function, per-type and per-file opt-outs from warnings.

mod attach;
mod directive;

pub use attach::{AllowTarget, Allows, TrackedAllow};
