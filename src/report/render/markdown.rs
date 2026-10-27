use std::fmt::Write as _;

use super::View;
use super::common::RenderHelpers;
use crate::report::model::Report;

impl Report {
    pub fn to_markdown(&self, view: View) -> String {
        let View { top, dirs } = view;
        [
            self.markdown_summary(),
            self.markdown_languages(),
            self.markdown_files(top),
            self.markdown_functions(top),
            if dirs {
                self.markdown_dirs(top)
            } else {
                String::new()
            },
            "\n".to_string(),
            self.warnings_block("## Refactor candidates"),
        ]
        .concat()
    }

    fn markdown_summary(&self) -> String {
        let t = &self.totals;
        format!(
            "# Repo size report\n\n**{} files**, {} lines ({} code, {} comments, {} blank), **{} tokens** (tokenizer: {})\n\n{}\n",
            t.files,
            t.lines,
            t.code,
            t.comments,
            t.blanks,
            t.tokens,
            self.tokenizer,
            self.skipped_line()
        )
    }

    fn markdown_languages(&self) -> String {
        let mut out = String::from(
            "## Languages\n\n| Language | Files | Lines | Code | Tokens |\n|---|---:|---:|---:|---:|\n",
        );
        for (name, l) in self.languages_by_tokens() {
            let _ = writeln!(
                out,
                "| {name} | {} | {} | {} | {} |",
                l.files, l.lines, l.code, l.tokens
            );
        }
        out
    }

    fn markdown_files(&self, top: usize) -> String {
        let mut out = format!(
            "\n## Files (top {top})\n\n| File | Lines | Code | Tokens | Fns |\n|---|---:|---:|---:|---:|\n"
        );
        for f in self.files.iter().take(top) {
            let _ = writeln!(
                out,
                "| `{}` | {} | {} | {} | {} |",
                f.path.display(),
                f.lines,
                f.code,
                f.tokens,
                RenderHelpers::fn_count(f)
            );
        }
        out
    }

    fn markdown_functions(&self, top: usize) -> String {
        if self.functions.is_empty() {
            return String::new();
        }
        let mut out = format!(
            "\n## Functions (top {top} by lines)\n\n| Function | Location | Lines | Tokens |\n|---|---|---:|---:|\n"
        );
        for f in self.functions.iter().take(top) {
            let _ = writeln!(
                out,
                "| `{}` | `{}:{}` | {} | {} |",
                f.name,
                f.path.display(),
                f.start_line,
                f.lines,
                f.tokens
            );
        }
        out
    }

    fn markdown_dirs(&self, top: usize) -> String {
        let mut out = format!(
            "\n## Folders (top {top} by tokens)\n\n| Folder | Files | Dirs | Total files | Lines | Tokens |\n|---|---:|---:|---:|---:|---:|\n"
        );
        for d in self.dirs_by_tokens().into_iter().take(top) {
            let _ = writeln!(
                out,
                "| `{}` | {} | {} | {} | {} | {} |",
                d.path.display(),
                d.files,
                d.dirs,
                d.total_files,
                d.lines,
                d.tokens
            );
        }
        out
    }
}
