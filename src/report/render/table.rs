use super::common::RenderHelpers;
use crate::report::model::Report;

impl Report {
    pub fn to_table(&self, top: usize) -> String {
        [
            self.table_summary(),
            self.table_languages(),
            self.table_files(top),
            self.table_functions(top),
            self.warnings_block("Warnings"),
        ]
        .concat()
    }

    fn table_summary(&self) -> String {
        let t = &self.totals;
        format!(
            "{} files, {} lines ({} code, {} comments, {} blank), {} tokens [{}]\n{}\n",
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

    fn table_languages(&self) -> String {
        let mut langs = RenderHelpers::new_table(&["Language", "Files", "Lines", "Code", "Tokens"]);
        for (name, l) in self.languages_by_tokens() {
            langs.add_row(vec![
                name.clone(),
                l.files.to_string(),
                l.lines.to_string(),
                l.code.to_string(),
                l.tokens.to_string(),
            ]);
        }
        format!("{langs}\n\n")
    }

    fn table_files(&self, top: usize) -> String {
        let mut files = RenderHelpers::new_table(&["File", "Lines", "Code", "Tokens", "Fns"]);
        for f in self.files.iter().take(top) {
            files.add_row(vec![
                f.path.display().to_string(),
                f.lines.to_string(),
                f.code.to_string(),
                f.tokens.to_string(),
                RenderHelpers::fn_count(f),
            ]);
        }
        format!("Files (top {top}):\n{files}\n\n")
    }

    fn table_functions(&self, top: usize) -> String {
        if self.functions.is_empty() {
            return String::new();
        }
        let mut fns = RenderHelpers::new_table(&["Function", "Location", "Lines", "Tokens"]);
        for f in self.functions.iter().take(top) {
            fns.add_row(vec![
                f.name.clone(),
                format!("{}:{}", f.path.display(), f.start_line),
                f.lines.to_string(),
                f.tokens.to_string(),
            ]);
        }
        format!("Functions (top {top} by lines):\n{fns}\n\n")
    }
}
