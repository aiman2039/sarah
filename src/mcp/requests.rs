use rmcp::schemars::{self, JsonSchema};
use serde::Deserialize;

#[derive(Debug, Clone, Copy, Default, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum ReportFormat {
    /// Markdown summary: totals, largest files and functions, warnings.
    #[default]
    Md,
    /// The full JSON report.
    Json,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ReportRequest {
    /// Directory to analyze (default: the server's working directory).
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub format: ReportFormat,
    /// Extra `sarah` CLI flags, e.g. `["--max-fn-lines", "60", "--changed-since", "origin/main"]`.
    #[serde(default)]
    pub args: Vec<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct WarningsRequest {
    /// Directory to analyze (default: the server's working directory).
    #[serde(default)]
    pub path: Option<String>,
    /// Only warnings for files changed since this git revision, e.g. `origin/main`.
    #[serde(default)]
    pub changed_since: Option<String>,
    /// Extra `sarah` CLI flags.
    #[serde(default)]
    pub args: Vec<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CheckFileRequest {
    /// The file to check, absolute or relative to `root`.
    pub file: String,
    /// Repository root whose `sarah.toml` applies (default: the server's working directory).
    #[serde(default)]
    pub root: Option<String>,
    /// Extra `sarah` CLI flags.
    #[serde(default)]
    pub args: Vec<String>,
}
