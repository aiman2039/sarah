use std::collections::HashMap;
use std::path::Path;

use serde_json::{Value, json};

use super::common::RenderHelpers;
use crate::count::FileStats;
use crate::report::model::Report;

/// A self-contained page: the treemap script and styles are inline, so it works offline.
const TEMPLATE: &str = include_str!("treemap.html");
const DATA_SLOT: &str = "__SARAH_DATA__";

impl Report {
    /// A treemap of files by tokens (or lines), with complexity heatmap, search and file inspector.
    pub fn to_html(&self) -> serde_json::Result<String> {
        let data = serde_json::to_string(&self.html_data())?;
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
            .map(|f| Self::html_file(f, &uri(&f.path), per_file.get(f.path.as_path()).copied()))
            .collect();
        let warnings: Vec<Value> = self
            .warnings
            .iter()
            .map(|w| {
                json!({ "path": uri(&w.path), "kind": w.kind, "message": w.message, "line": w.line })
            })
            .collect();
        json!({
            "tokenizer": self.tokenizer,
            "totals": self.totals,
            "files": files,
            "warnings": warnings,
        })
    }

    fn html_file(f: &FileStats, path_uri: &str, warnings: Option<usize>) -> Value {
        let max_cog = f
            .functions
            .iter()
            .flatten()
            .map(|fn_s| fn_s.cognitive.0)
            .max()
            .unwrap_or(0);
        let fns: Vec<Value> = f
            .functions
            .iter()
            .flatten()
            .map(|fn_s| {
                json!({
                    "name": fn_s.name,
                    "line": fn_s.start_line,
                    "lines": fn_s.lines,
                    "tokens": fn_s.tokens,
                    "depth": fn_s.depth,
                    "cognitive": fn_s.cognitive,
                })
            })
            .collect();
        json!({
            "path": path_uri,
            "language": f.language,
            "tokens": f.tokens,
            "lines": f.lines,
            "cognitive": max_cog,
            "warnings": warnings.unwrap_or_default(),
            "fns": fns,
        })
    }
}
