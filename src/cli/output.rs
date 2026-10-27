/// The binary's only writer to stdout and stderr (rust.md: one output module).
pub struct CliOutput;

impl CliOutput {
    #[expect(
        clippy::print_stdout,
        reason = "the rendered report is the CLI's output"
    )]
    pub fn print_report(rendered: &str) {
        println!("{rendered}");
    }

    #[expect(clippy::print_stderr, reason = "fatal errors go to stderr")]
    pub fn print_error(err: &anyhow::Error) {
        eprintln!("Error: {err:?}");
    }
}
