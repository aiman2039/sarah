/// The binary's only writer to stdout (rust.md: one output module).
pub struct CliOutput;

impl CliOutput {
    #[expect(
        clippy::print_stdout,
        reason = "the rendered report is the CLI's output"
    )]
    pub fn print_report(rendered: &str) {
        println!("{rendered}");
    }
}
