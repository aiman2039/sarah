use std::path::Path;

use anyhow::{Context, Result};
use serde::Deserialize;

use super::naming::Naming;
use super::preset::Preset;
use crate::config::{DirPattern, NamePatterns};
use crate::units::{Depth, Tokens};

/// A `[[folder]]` entry in `sarah.toml`. Counts are direct children, except
/// `max_tokens`, which covers the whole subtree.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FolderRule {
    pub path: String,
    pub max_files: Option<usize>,
    pub max_dirs: Option<usize>,
    pub max_depth: Option<Depth>,
    pub max_tokens: Option<Tokens>,
    /// Minimum direct entries (files plus folders).
    pub min_files: Option<usize>,
    /// Each glob must match at least one file in the folder.
    pub require: Option<Vec<String>>,
    /// No file or folder name in the folder may match these globs.
    pub forbid: Option<Vec<String>>,
    /// Sub-folder names must match one of these globs.
    pub allow_only: Option<Vec<String>>,
    pub naming: Option<Naming>,
    /// Forbid `x.rs` next to `x/` (one module root per topic).
    pub no_sibling_module: Option<bool>,
}

struct CompiledRule {
    dir: DirPattern,
    rule: FolderRule,
    require: Option<NamePatterns>,
    forbid: Option<NamePatterns>,
    allow_only: Option<NamePatterns>,
}

/// All folder rules in order: the preset's first, then `[[folder]]`.
pub struct FolderRules(Vec<CompiledRule>);

/// The rules that apply to one folder, merged; later rules win field by field.
#[derive(Default)]
pub struct Policy<'a> {
    pub max_files: Option<usize>,
    pub max_dirs: Option<usize>,
    pub max_depth: Option<Depth>,
    pub max_tokens: Option<Tokens>,
    pub min_files: Option<usize>,
    pub require: Option<&'a NamePatterns>,
    pub forbid: Option<&'a NamePatterns>,
    pub allow_only: Option<&'a NamePatterns>,
    pub naming: Option<Naming>,
    pub no_sibling_module: bool,
}

impl FolderRules {
    pub fn new(preset: Option<Preset>, rules: Vec<FolderRule>) -> Result<Self> {
        preset
            .map(Preset::rules)
            .unwrap_or_default()
            .into_iter()
            .chain(rules)
            .map(CompiledRule::new)
            .collect::<Result<_>>()
            .map(Self)
    }

    /// `None` when no rule matches the folder.
    pub fn policy_for(&self, dir: &Path) -> Option<Policy<'_>> {
        let mut matching = self.0.iter().filter(|c| c.dir.matches(dir)).peekable();
        matching.peek()?;
        let mut policy = Policy::default();
        for compiled in matching {
            compiled.merge_into(&mut policy);
        }
        Some(policy)
    }
}

impl CompiledRule {
    fn new(rule: FolderRule) -> Result<Self> {
        let context = || format!("in [[folder]] path = {:?} of sarah.toml", rule.path);
        let patterns = |globs: &Option<Vec<String>>| {
            globs
                .as_deref()
                .map(NamePatterns::new)
                .transpose()
                .with_context(context)
        };
        Ok(Self {
            dir: DirPattern::new(&rule.path).with_context(context)?,
            require: patterns(&rule.require)?,
            forbid: patterns(&rule.forbid)?,
            allow_only: patterns(&rule.allow_only)?,
            rule,
        })
    }

    fn merge_into<'a>(&'a self, policy: &mut Policy<'a>) {
        let FolderRule {
            max_files,
            max_dirs,
            max_depth,
            max_tokens,
            min_files,
            naming,
            no_sibling_module,
            ..
        } = &self.rule;
        policy.max_files = max_files.or(policy.max_files);
        policy.max_dirs = max_dirs.or(policy.max_dirs);
        policy.max_depth = max_depth.or(policy.max_depth);
        policy.max_tokens = max_tokens.or(policy.max_tokens);
        policy.min_files = min_files.or(policy.min_files);
        policy.naming = naming.or(policy.naming);
        policy.no_sibling_module = no_sibling_module.unwrap_or(policy.no_sibling_module);
        policy.require = self.require.as_ref().or(policy.require);
        policy.forbid = self.forbid.as_ref().or(policy.forbid);
        policy.allow_only = self.allow_only.as_ref().or(policy.allow_only);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn later_rules_win_field_by_field() {
        let rules = FolderRules::new(
            Some(Preset::Rust),
            vec![FolderRule {
                path: "src/gen/**".into(),
                max_files: Some(50),
                require: Some(vec![]),
                ..FolderRule::default()
            }],
        )
        .unwrap();
        let generated = rules.policy_for(Path::new("src/gen/x")).unwrap();
        assert_eq!(generated.max_files, Some(50));
        assert_eq!(generated.naming, Some(Naming::Snake));
        assert!(generated.no_sibling_module);
        assert_eq!(generated.require.unwrap().iter().count(), 0);

        let other = rules.policy_for(Path::new("src/a")).unwrap();
        assert_eq!(other.max_files, None);
        assert_eq!(other.require.unwrap().find("mod.rs"), Some("mod.rs"));
        assert!(rules.policy_for(Path::new("docs")).is_none());
    }
}
