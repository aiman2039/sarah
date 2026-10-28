//! Command-line interface: arguments, the command flows, and the one place that writes to stdout.

mod args;
mod command;
mod output;
mod run;
mod scan;
mod tune;

pub use args::{Args, CheckArgs};
