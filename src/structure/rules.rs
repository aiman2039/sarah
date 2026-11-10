use std::path::Path;

use anyhow::{Context, Result};
use serde::Deserialize;

use super::naming::Naming;
use super::preset::Preset;
use crate::config::{DirPattern, NamePatterns};
use crate::units::{Depth, Lines, Tokens};

/// One `max_*` key of a `[[folder]]` rule that `sarah init` and `sarah ratchet` manage.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FolderLimitKey {
    Files,
    Dirs,
    Depth,
    Lines,
    Tokens,
}

impl FolderLimitKey {
    pub const ALL: [Self; 5] = [
        Self::Files,
        Self::Dirs,
        Self::Depth,
        Self::Lines,
        Self::Tokens,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Self::Files => "max_files",
            Self::Dirs => "max_dirs",
            Self::Depth => "max_depth",
            Self::Lines => "max_lines",
            Self::Tokens => "max_tokens",
        }
    }
}

/// A `[[folder]]` entry in `sarah.toml`. Counts are direct children, except
/// `max_lines` and `max_tokens`, which cover the whole subtree.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FolderRule {
    pub path: String,
    pub max_files: Option<usize>,
    pub max_dirs: Option<usize>,
    pub max_depth: Option<Depth>,
    pub max_lines: Option<Lines>,
    pub max_tokens: Option<Tokens>,
    /// Minimum direct entries (files plus folders).
    pub min_files: Option<usize>,
    /// Each glob must match at least one file in the folder.
    pub require: Option<Vec<String>>,
    /// No file or folder name in the folder may match these globs.
    pub forbid: Option<Vec<String>>,
    /// No import in files of the folder may match these globs.
    pub forbid_imports: Option<Vec<String>>,
    /// Sub-folder names must match one of these globs.
    pub allow_only: Option<Vec<String>>,
    pub naming: Option<Naming>,
    /// Forbid `x.<ext>` next to `x/` (one module root per topic).
    pub no_sibling_module: Option<bool>,
}

struct CompiledRule {
    dir: DirPattern,
    rule: FolderRule,
    require: Option<NamePatterns>,
    forbid: Option<NamePatterns>,
    forbid_imports: Option<NamePatterns>,
    allow_only: Option<NamePatterns>,
}

/// All folder rules in order: the preset's first, then explicit `[[folder]]` rules.
pub struct FolderRules {
    preset_count: usize,
    rules: Vec<CompiledRule>,
}

/// The rules that apply to one folder, merged; later rules win field by field.
#[derive(Default)]
pub struct Policy<'a> {
    pub max_files: Option<usize>,
    pub max_dirs: Option<usize>,
    pub max_depth: Option<Depth>,
    pub max_lines: Option<Lines>,
    pub max_tokens: Option<Tokens>,
    pub min_files: Option<usize>,
    pub require: Option<&'a NamePatterns>,
    pub forbid: Option<&'a NamePatterns>,
    pub forbid_imports: Option<&'a NamePatterns>,
    pub allow_only: Option<&'a NamePatterns>,
    pub naming: Option<Naming>,
    pub no_sibling_module: bool,
}

impl FolderRule {
    pub fn get(&self, key: FolderLimitKey) -> Option<usize> {
        match key {
            FolderLimitKey::Files => self.max_files,
            FolderLimitKey::Dirs => self.max_dirs,
            FolderLimitKey::Depth => self.max_depth.map(|v| v.0),
            FolderLimitKey::Lines => self.max_lines.map(|v| v.0),
            FolderLimitKey::Tokens => self.max_tokens.map(|v| v.0),
        }
    }
}

impl FolderRules {
    pub fn new(preset: Option<Preset>, rules: Vec<FolderRule>) -> Result<Self> {
        let preset_rules = preset.map(Preset::rules).unwrap_or_default();
        let preset_count = preset_rules.len();
        let compiled = preset_rules
            .into_iter()
            .chain(rules)
            .map(CompiledRule::new)
            .collect::<Result<_>>()?;
        Ok(Self {
            preset_count,
            rules: compiled,
        })
    }

    /// `None` when no rule matches the folder.
    pub fn policy_for(&self, dir: &Path) -> Option<Policy<'_>> {
        let mut matching = self.rules.iter().filter(|c| c.dir.matches(dir)).peekable();
        matching.peek()?;
        let mut policy = Policy::default();
        for compiled in matching {
            compiled.merge_into(&mut policy);
        }
        Some(policy)
    }

    /// Index of the last explicit `[[folder]]` rule that matches `dir` and sets `key`.
    pub fn governing_rule(&self, dir: &Path, key: FolderLimitKey) -> Option<usize> {
        self.rules
            .get(self.preset_count..)?
            .iter()
            .rposition(|c| c.rule.get(key).is_some() && c.dir.matches(dir))
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
            forbid_imports: patterns(&rule.forbid_imports)?,
            allow_only: patterns(&rule.allow_only)?,
            rule,
        })
    }

    fn merge_into<'a>(&'a self, policy: &mut Policy<'a>) {
        let r = &self.rule;
        policy.max_files = r.max_files.or(policy.max_files);
        policy.max_dirs = r.max_dirs.or(policy.max_dirs);
        policy.max_depth = r.max_depth.or(policy.max_depth);
        policy.max_lines = r.max_lines.or(policy.max_lines);
        policy.max_tokens = r.max_tokens.or(policy.max_tokens);
        policy.min_files = r.min_files.or(policy.min_files);
        policy.naming = r.naming.or(policy.naming);
        policy.no_sibling_module = r.no_sibling_module.unwrap_or(policy.no_sibling_module);
        policy.require = self.require.as_ref().or(policy.require);
        policy.forbid = self.forbid.as_ref().or(policy.forbid);
        policy.forbid_imports = self.forbid_imports.as_ref().or(policy.forbid_imports);
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
        assert_eq!(
            rules.governing_rule(Path::new("src/gen/x"), FolderLimitKey::Files),
            Some(0)
        );

        let other = rules.policy_for(Path::new("src/a")).unwrap();
        assert_eq!(other.max_files, None);
        assert_eq!(other.require.unwrap().find("mod.rs"), Some("mod.rs"));
        assert!(rules.policy_for(Path::new("docs")).is_none());
    }
}
