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

/// Any `src` folder, at the root or in a package of a monorepo.
const SRC: &str = "**/src";
/// Every folder below any `src` folder.
const IN_SRC: &str = "**/src/**";

/// Catch-all names that hide what a module is about.
const CATCH_ALL: &[&str] = &[
    "util", "util.*", "utils", "utils.*", "misc", "misc.*", "stuff", "stuff.*",
];

impl Preset {
    pub fn rules(self) -> Vec<FolderRule> {
        let forbid = Some(CATCH_ALL.iter().map(ToString::to_string).collect());
        match self {
            Self::Rust => vec![FolderRule {
                path: IN_SRC.into(),
                require: Some(vec!["mod.rs".into()]),
                naming: Some(Naming::Snake),
                no_sibling_module: Some(true),
                forbid,
                ..FolderRule::default()
            }],
            Self::Python => vec![FolderRule {
                path: IN_SRC.into(),
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
            // `src` itself counts here: a flat `src` is the case this limit is for.
            Self::Ts => [SRC, IN_SRC]
                .into_iter()
                .map(|path| FolderRule {
                    path: path.into(),
                    forbid: forbid.clone(),
                    max_files: Some(20),
                    ..FolderRule::default()
                })
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use crate::config::DirPattern;

    fn selected(preset: Preset, dir: &str) -> bool {
        preset
            .rules()
            .iter()
            .any(|rule| DirPattern::new(&rule.path).unwrap().matches(Path::new(dir)))
    }

    #[test]
    fn ts_preset_covers_src_at_any_depth() {
        for dir in ["src", "src/chart", "apps/web/src", "apps/web/src/chart"] {
            assert!(selected(Preset::Ts, dir), "{dir}");
        }
        assert!(!selected(Preset::Ts, "apps/web/docs"));
    }

    #[test]
    fn rust_preset_skips_the_crate_root() {
        assert!(selected(Preset::Rust, "crates/a/src/net"));
        assert!(!selected(Preset::Rust, "crates/a/src"));
    }
}
