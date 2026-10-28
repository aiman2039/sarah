use std::process::ExitCode;

use super::args::{Args, Command};
use super::output::CliOutput;
use crate::mcp::McpServer;

/// Exit code for errors; 1 means "warnings with --fail-on-warn".
const EXIT_ERROR: u8 = 2;

impl Args {
    /// The whole program; errors are printed and exit with code 2.
    pub fn run(&self) -> ExitCode {
        let result = match &self.command {
            None => self.check.run(),
            Some(Command::Init(walk)) => walk.init(),
            Some(Command::Ratchet(ratchet)) => ratchet.run(),
            Some(Command::Mcp) => McpServer::run_stdio(),
        };
        result.unwrap_or_else(|err| {
            CliOutput::print_error(&err);
            ExitCode::from(EXIT_ERROR)
        })
    }
}
