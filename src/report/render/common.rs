use std::cmp::Reverse;
use std::fmt::Write as _;

use comfy_table::{Table, presets::UTF8_FULL_CONDENSED};

use crate::count::FileStats;
use crate::report::model::{Report, Totals};

impl Report {
    /// The "Skipped: ..." line, or nothing when no entry was skipped.
    pub(super) fn skipped_line(&self) -> String {
        let s = &self.skipped;
        if s.total() == 0 {
            return String::new();
        }
        format!(
            "Skipped: {} broken links, {} links outside root, {} directory links, {} duplicate/excluded link targets, {} unreadable\n",
            s.broken_links, s.outside_root, s.dir_links, s.duplicates, s.errors
        )
    }

    pub(super) fn languages_by_tokens(&self) -> Vec<(&String, &Totals)> {
        let mut langs: Vec<_> = self.by_language.iter().collect();
        langs.sort_by_key(|(_, l)| Reverse(l.tokens));
        langs
    }

    pub(super) fn warnings_block(&self, heading: &str) -> String {
        if self.warnings.is_empty() {
            return format!("{heading}: none\n");
        }
        let mut out = format!("{heading} ({}):\n\n", self.warnings.len());
        for w in &self.warnings {
            let _ = writeln!(out, "- {}: {}", w.path.display(), w.message);
        }
        out
    }
}

/// Stateless helpers shared by the table and markdown renderers.
pub(super) struct RenderHelpers;

impl RenderHelpers {
    pub(super) fn new_table(header: &[&str]) -> Table {
        let mut table = Table::new();
        table.load_style(UTF8_FULL_CONDENSED).set_header(header);
        table
    }

    pub(super) fn fn_count(f: &FileStats) -> String {
        f.functions
            .as_ref()
            .map_or("-".to_string(), |v| v.len().to_string())
    }
}
