use serde::Deserialize;

use super::naming::Naming;
use super::rules::FolderRule;

/// Built-in folder rules for a language, applied before the `[[folder]]` rules.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Preset {
    Rust,
    Python,
    Go,
    Ts,
}

/// Catch-all names that hide what a module is about.
const CATCH_ALL: &[&str] = &[
    "util", "util.*", "utils", "utils.*", "misc", "misc.*", "stuff", "stuff.*",
];

impl Preset {
    pub fn rules(self) -> Vec<FolderRule> {
        let forbid = Some(CATCH_ALL.iter().map(ToString::to_string).collect());
        match self {
            Self::Rust => vec![FolderRule {
                path: "src/**".into(),
                require: Some(vec!["mod.rs".into()]),
                naming: Some(Naming::Snake),
                no_sibling_module: Some(true),
                forbid,
                ..FolderRule::default()
            }],
            Self::Python => vec![FolderRule {
                path: "src/**".into(),
                require: Some(vec!["__init__.py".into()]),
                naming: Some(Naming::Snake),
                forbid,
                ..FolderRule::default()
            }],
            Self::Go => vec![FolderRule {
                path: "**/*".into(),
                forbid,
                max_files: Some(30),
                ..FolderRule::default()
            }],
            Self::Ts => vec![FolderRule {
                path: "src/**".into(),
                forbid,
                max_files: Some(20),
                ..FolderRule::default()
            }],
        }
    }
}
