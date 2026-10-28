use std::collections::HashMap;
use std::path::Path;

use serde_json::{Value, json};

use super::common::RenderHelpers;
use crate::report::model::Report;

/// A self-contained page: the treemap script and styles are inline, so it works offline.
const TEMPLATE: &str = include_str!("treemap.html");
const DATA_SLOT: &str = "__SARAH_DATA__";

impl Report {
    /// A treemap of files by tokens (or lines), with folders to zoom into and the warnings.
    pub fn to_html(&self) -> serde_json::Result<String> {
        let data = serde_json::to_string(&self.html_data())?;
        // `</script>` inside the JSON would end the data block early.
        Ok(TEMPLATE.replace(DATA_SLOT, &data.replace("</", "<\\/")))
    }

    fn html_data(&self) -> Value {
        let mut per_file: HashMap<&Path, usize> = HashMap::new();
        for w in &self.warnings {
            let count = per_file.entry(w.path.as_path()).or_default();
            *count = count.saturating_add(1);
        }
        let uri = |path: &Path| RenderHelpers::repo_uri(Path::new("."), path);
        let files: Vec<Value> = self
            .files
            .iter()
            .map(|f| {
                json!({
                    "path": uri(&f.path),
                    "language": f.language,
                    "tokens": f.tokens,
                    "lines": f.lines,
                    "warnings": per_file.get(f.path.as_path()).copied().unwrap_or_default(),
                })
            })
            .collect();
        let warnings: Vec<Value> = self
            .warnings
            .iter()
            .map(|w| json!({ "path": uri(&w.path), "kind": w.kind, "message": w.message, "line": w.line }))
            .collect();
        json!({
            "tokenizer": self.tokenizer,
            "totals": self.totals,
            "files": files,
            "warnings": warnings,
        })
    }
}
