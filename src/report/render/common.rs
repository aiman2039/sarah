use std::cmp::Reverse;
use std::fmt::Write as _;
use std::path::{Component, Path};

use comfy_table::{Table, presets::UTF8_FULL_CONDENSED};

use crate::count::FileStats;
use crate::report::model::{Report, Totals};
use crate::structure::DirStats;

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
        let mut out = if self.warnings.is_empty() {
            format!("{heading}: none\n")
        } else {
            let mut out = format!("{heading} ({}):\n\n", self.warnings.len());
            for w in &self.warnings {
                let _ = writeln!(out, "- {}: {}", w.path.display(), w.message);
            }
            out
        };
        if let Some(hidden) = self.baselined {
            let _ = writeln!(out, "\n{hidden} known warnings hidden by --baseline");
        }
        out
    }

    /// Folders by subtree tokens, largest first.
    pub(super) fn dirs_by_tokens(&self) -> Vec<&DirStats> {
        let mut dirs: Vec<_> = self.directories.iter().collect();
        dirs.sort_by_key(|d| Reverse(d.tokens));
        dirs
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

    /// `path` as a forward-slash URI relative to the repo, given the analyzed `root`.
    pub(super) fn repo_uri(root: &Path, path: &Path) -> String {
        let joined = if root == Path::new(".") || root.as_os_str().is_empty() {
            path.to_path_buf()
        } else {
            root.join(path)
        };
        let parts: Vec<_> = joined
            .components()
            .filter(|c| !matches!(c, Component::CurDir))
            .map(|c| c.as_os_str().to_string_lossy().into_owned())
            .collect();
        if parts.is_empty() {
            ".".to_string()
        } else {
            parts.join("/")
        }
    }

    pub(super) fn fn_count(f: &FileStats) -> String {
        f.functions
            .as_ref()
            .map_or("-".to_string(), |v| v.len().to_string())
    }
}
