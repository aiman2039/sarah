use std::collections::HashMap;

use crate::config::{LimitKey, LimitRules};
use crate::count::FileStats;
use crate::functions::{FnStats, TypeStats};
use crate::report::WarningKind;
use crate::structure::{DirStats, DirTree, FolderLimitKey, FolderRules};

/// Measured maximums per `[[files]]` and `[[folder]]` rule (`None`: no rule sets the key).
/// Values exempted by `sarah-allow` comments are left out.
pub struct Maxima {
    files: HashMap<(Option<usize>, LimitKey), usize>,
    folders: HashMap<(Option<usize>, FolderLimitKey), usize>,
}

impl Maxima {
    pub fn of(files: &[FileStats], file_rules: &LimitRules, folder_rules: &FolderRules) -> Self {
        Self {
            files: Self::of_files(files, file_rules),
            folders: Self::of_folders(&DirTree::build(files), folder_rules),
        }
    }

    pub fn get_file(&self, rule: Option<usize>, key: LimitKey) -> Option<usize> {
        self.files.get(&(rule, key)).copied()
    }

    pub fn get_folder(&self, rule: Option<usize>, key: FolderLimitKey) -> Option<usize> {
        self.folders.get(&(rule, key)).copied()
    }

    fn of_files(
        files: &[FileStats],
        rules: &LimitRules,
    ) -> HashMap<(Option<usize>, LimitKey), usize> {
        let mut maxima = HashMap::new();
        for file in files {
            for key in LimitKey::ALL {
                let rule = rules.governing_rule(&file.path, key);
                if let Some(value) = Self::file_value(file, key) {
                    let max = maxima.entry((rule, key)).or_insert(0);
                    *max = (*max).max(value);
                }
            }
        }
        maxima
    }

    fn of_folders(
        tree: &DirTree,
        rules: &FolderRules,
    ) -> HashMap<(Option<usize>, FolderLimitKey), usize> {
        let mut maxima = HashMap::new();
        for node in tree.nodes() {
            for key in FolderLimitKey::ALL {
                let rule = rules.governing_rule(&node.stats.path, key);
                let value = Self::dir_value(&node.stats, key);
                let max = maxima.entry((rule, key)).or_insert(0);
                *max = (*max).max(value);
            }
        }
        maxima
    }

    fn dir_value(stats: &DirStats, key: FolderLimitKey) -> usize {
        match key {
            FolderLimitKey::Files => stats.files,
            FolderLimitKey::Dirs => stats.dirs,
            FolderLimitKey::Depth => stats.depth.0,
            FolderLimitKey::Lines => stats.lines.0,
            FolderLimitKey::Tokens => stats.tokens.0,
        }
    }

    fn file_value(file: &FileStats, key: LimitKey) -> Option<usize> {
        let kind = Self::warning_kind(key);
        if file.allows.file.contains(&kind) {
            return None;
        }
        match key {
            LimitKey::FileTokens => Some(file.tokens.0),
            LimitKey::FileLines => Some(file.lines.0),
            LimitKey::FileFns => file.functions.as_ref().map(Vec::len),
            LimitKey::TypeFields | LimitKey::TypeMethods => file
                .types
                .iter()
                .filter(|ty| !ty.allow.contains(&kind))
                .map(|ty| Self::type_value(ty, key))
                .max(),
            _ => file
                .functions
                .iter()
                .flatten()
                .filter(|func| !func.allow.contains(&kind))
                .filter_map(|func| Self::fn_value(func, key))
                .max(),
        }
    }

    fn type_value(ty: &TypeStats, key: LimitKey) -> usize {
        match key {
            LimitKey::TypeFields => ty.fields.0,
            _ => ty.methods.0,
        }
    }

    fn fn_value(func: &FnStats, key: LimitKey) -> Option<usize> {
        match key {
            LimitKey::FnLines => Some(func.lines.0),
            LimitKey::FnTokens => Some(func.tokens.0),
            LimitKey::FnDepth => Some(func.depth.0),
            LimitKey::FnParams => func.params.map(|p| p.0),
            LimitKey::FnBoolParams => func.bool_params.map(|p| p.0),
            LimitKey::FnCyclomatic => Some(func.cyclomatic.0),
            LimitKey::FnCognitive => Some(func.cognitive.0),
            _ => None,
        }
    }

    fn warning_kind(key: LimitKey) -> WarningKind {
        match key {
            LimitKey::FileTokens | LimitKey::FileLines => WarningKind::LargeFile,
            LimitKey::FileFns => WarningKind::TooManyFunctions,
            LimitKey::FnLines | LimitKey::FnTokens => WarningKind::LargeFunction,
            LimitKey::FnDepth => WarningKind::DeepFunction,
            LimitKey::FnParams => WarningKind::TooManyParams,
            LimitKey::FnBoolParams => WarningKind::TooManyBoolParams,
            LimitKey::FnCyclomatic | LimitKey::FnCognitive => WarningKind::ComplexFunction,
            LimitKey::TypeFields | LimitKey::TypeMethods => WarningKind::LargeType,
        }
    }
}
