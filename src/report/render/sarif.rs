use std::collections::BTreeSet;
use std::path::Path;

use serde_json::{Value, json};

use super::common::RenderHelpers;
use crate::report::model::{Report, Warning};

impl Report {
    /// SARIF 2.1.0 for GitHub code scanning. `root` is the analyzed path as given.
    pub fn to_sarif(&self, root: &Path) -> serde_json::Result<String> {
        let rules: BTreeSet<String> = self.warnings.iter().map(|w| w.kind.to_string()).collect();
        let rules: Vec<Value> = rules
            .into_iter()
            .map(|id| json!({ "id": id, "shortDescription": { "text": id } }))
            .collect();
        let results: Vec<Value> = self
            .warnings
            .iter()
            .map(|w| Self::sarif_result(root, w))
            .collect();
        serde_json::to_string_pretty(&json!({
            "$schema": "https://json.schemastore.org/sarif-2.1.0.json",
            "version": "2.1.0",
            "runs": [{
                "tool": { "driver": {
                    "name": "sarah",
                    "version": env!("CARGO_PKG_VERSION"),
                    "informationUri": env!("CARGO_PKG_REPOSITORY"),
                    "rules": rules,
                }},
                "results": results,
            }],
        }))
    }

    fn sarif_result(root: &Path, w: &Warning) -> Value {
        let uri = RenderHelpers::repo_uri(root, &w.path);
        let location = match w.line {
            Some(line) => json!({
                "artifactLocation": { "uri": uri },
                "region": { "startLine": line },
            }),
            None => json!({ "artifactLocation": { "uri": uri } }),
        };
        json!({
            "ruleId": w.kind.to_string(),
            "level": "warning",
            "message": { "text": w.message },
            "locations": [{ "physicalLocation": location }],
        })
    }
}
