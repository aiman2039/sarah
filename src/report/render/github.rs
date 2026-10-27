use std::fmt::Write as _;
use std::path::Path;

use super::common::RenderHelpers;
use crate::report::model::Report;

impl Report {
    /// GitHub Actions workflow commands: one `::warning` annotation per warning.
    /// `root` is the analyzed path as given, so paths resolve from the repo root.
    pub fn to_github(&self, root: &Path) -> String {
        let mut out = String::new();
        for w in &self.warnings {
            let mut props = format!(
                "file={}",
                Self::escape_property(&RenderHelpers::repo_uri(root, &w.path))
            );
            if let Some(line) = w.line {
                let _ = write!(props, ",line={line}");
            }
            let title = Self::escape_property(&format!("sarah {}", w.kind));
            let _ = writeln!(
                out,
                "::warning {props},title={title}::{}",
                Self::escape_data(&w.message)
            );
        }
        out
    }

    fn escape_data(text: &str) -> String {
        text.replace('%', "%25")
            .replace('\r', "%0D")
            .replace('\n', "%0A")
    }

    fn escape_property(text: &str) -> String {
        Self::escape_data(text)
            .replace(':', "%3A")
            .replace(',', "%2C")
    }
}
