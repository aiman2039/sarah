use serde::Deserialize;

use super::naming::Naming;
use super::rules::FolderRule;
use crate::units::Depth;

/// Built-in folder rules for a language, applied before the `[[folder]]` rules.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Preset {
    Rust,
    Python,
    Go,
    Ts,
    Js,
    Java,
    Kotlin,
    Csharp,
    Swift,
    Ruby,
    Php,
    C,
    Cpp,
    Scala,
    Elixir,
    Zig,
}

const SRC: &str = "**/src";
const IN_SRC: &str = "**/src/**";
const DEFAULT_MAX_DIRS: usize = 15;
const DEFAULT_REL_DEPTH: Depth = Depth(5);

const CATCH_ALL: &[&str] = &[
    "util", "util.*", "utils", "utils.*", "misc", "misc.*", "stuff", "stuff.*",
];

impl Preset {
    pub fn rules(self) -> Vec<FolderRule> {
        let forbid = || Some(CATCH_ALL.iter().map(ToString::to_string).collect());
        match self {
            Self::Rust => Self::rust_rules(forbid()),
            Self::Python => vec![FolderRule {
                path: IN_SRC.into(),
                max_files: Some(25),
                max_dirs: Some(DEFAULT_MAX_DIRS),
                max_relative_depth: Some(DEFAULT_REL_DEPTH),
                require: Some(vec!["__init__.py".into()]),
                naming: Some(Naming::Snake),
                forbid: forbid(),
                ..FolderRule::default()
            }],
            Self::Go => Self::for_paths(&["**/*"], None, 30),
            Self::Ts | Self::Js => Self::for_paths(&[SRC, IN_SRC], None, 20),
            Self::Java | Self::Kotlin | Self::Scala => Self::for_paths(&[IN_SRC], None, 25),
            Self::Csharp => Self::for_paths(&["**/*"], Some(Naming::Pascal), 25),
            Self::Swift => Self::for_paths(&["**/Sources/**", IN_SRC], None, 25),
            Self::Ruby => Self::for_paths(&["**/lib/**", "**/app/**"], Some(Naming::Snake), 25),
            Self::Php => Self::for_paths(&[IN_SRC, "**/app/**"], None, 25),
            Self::C | Self::Cpp => Self::for_paths(&[IN_SRC, "**/include/**"], None, 30),
            Self::Elixir => Self::for_paths(&["**/lib/**"], Some(Naming::Snake), 25),
            Self::Zig => Self::for_paths(&[IN_SRC], Some(Naming::Snake), 25),
        }
    }

    fn rust_rules(forbid: Option<Vec<String>>) -> Vec<FolderRule> {
        let bin_override = |path: &str| FolderRule {
            path: path.into(),
            require: Some(Vec::new()),
            ..FolderRule::default()
        };
        vec![
            FolderRule {
                path: IN_SRC.into(),
                max_files: Some(20),
                max_dirs: Some(DEFAULT_MAX_DIRS),
                max_relative_depth: Some(DEFAULT_REL_DEPTH),
                require: Some(vec!["mod.rs".into()]),
                naming: Some(Naming::Snake),
                no_sibling_module: Some(true),
                forbid,
                ..FolderRule::default()
            },
            bin_override("**/src/bin"),
            bin_override("**/src/bin/*"),
        ]
    }

    fn for_paths(paths: &[&str], naming: Option<Naming>, max_files: usize) -> Vec<FolderRule> {
        let forbid: Vec<String> = CATCH_ALL.iter().map(ToString::to_string).collect();
        paths
            .iter()
            .map(|path| FolderRule {
                path: (*path).into(),
                forbid: Some(forbid.clone()),
                naming,
                max_files: Some(max_files),
                max_dirs: Some(DEFAULT_MAX_DIRS),
                max_relative_depth: Some(DEFAULT_REL_DEPTH),
                ..FolderRule::default()
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use crate::structure::FolderRules;

    fn has_policy(preset: Preset, dir: &str) -> bool {
        FolderRules::new(Some(preset), Vec::new())
            .unwrap()
            .policy_for(Path::new(dir))
            .is_some()
    }

    #[test]
    fn ts_preset_covers_src_at_any_depth() {
        for dir in ["src", "src/chart", "apps/web/src", "apps/web/src/chart"] {
            assert!(has_policy(Preset::Ts, dir), "{dir}");
        }
        assert!(!has_policy(Preset::Ts, "apps/web/docs"));
    }

    #[test]
    fn rust_preset_sets_limits_and_relaxes_src_bin() {
        let rules = FolderRules::new(Some(Preset::Rust), Vec::new()).unwrap();
        assert!(rules.policy_for(Path::new("crates/a/src")).is_none());
        let net = rules.policy_for(Path::new("crates/a/src/net")).unwrap();
        assert_eq!(net.require.unwrap().find("mod.rs"), Some("mod.rs"));
        assert_eq!(net.max_files, Some(20));
        assert_eq!(net.max_relative_depth, Some((Depth(5), Some("src"))));

        let bin = rules.policy_for(Path::new("src/bin")).unwrap();
        assert_eq!(bin.require.unwrap().iter().count(), 0);
    }
}
