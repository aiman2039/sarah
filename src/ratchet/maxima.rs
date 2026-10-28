use std::collections::HashMap;

use crate::config::{LimitKey, LimitRules};
use crate::count::FileStats;
use crate::functions::FnStats;
use crate::report::WarningKind;

/// The largest measured value per limit key, grouped by the `[[files]]` rule whose value
/// applies (`None`: no rule sets the key, so the default does). Values that a
/// `sarah-allow` comment exempts are left out.
pub struct Maxima(HashMap<(Option<usize>, LimitKey), usize>);

impl Maxima {
    pub fn of(files: &[FileStats], rules: &LimitRules) -> Self {
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
        Self(maxima)
    }

    pub fn get(&self, rule: Option<usize>, key: LimitKey) -> Option<usize> {
        self.0.get(&(rule, key)).copied()
    }

    /// The file's own size for file keys, its largest function for function keys.
    fn file_value(file: &FileStats, key: LimitKey) -> Option<usize> {
        let kind = Self::warning_kind(key);
        if file.allows.file.contains(&kind) {
            return None;
        }
        match key {
            LimitKey::FileTokens => Some(file.tokens.0),
            LimitKey::FileLines => Some(file.lines.0),
            _ => file
                .functions
                .iter()
                .flatten()
                .filter(|func| !func.allow.contains(&kind))
                .filter_map(|func| Self::fn_value(func, key))
                .max(),
        }
    }

    fn fn_value(func: &FnStats, key: LimitKey) -> Option<usize> {
        match key {
            LimitKey::FileTokens | LimitKey::FileLines => None,
            LimitKey::FnLines => Some(func.lines.0),
            LimitKey::FnTokens => Some(func.tokens.0),
            LimitKey::FnDepth => Some(func.depth.0),
            LimitKey::FnParams => func.params.map(|p| p.0),
            LimitKey::FnCyclomatic => Some(func.cyclomatic.0),
            LimitKey::FnCognitive => Some(func.cognitive.0),
        }
    }

    /// The warning a value over this key raises (and that `sarah-allow` can turn off).
    fn warning_kind(key: LimitKey) -> WarningKind {
        match key {
            LimitKey::FileTokens | LimitKey::FileLines => WarningKind::LargeFile,
            LimitKey::FnLines | LimitKey::FnTokens => WarningKind::LargeFunction,
            LimitKey::FnDepth => WarningKind::DeepFunction,
            LimitKey::FnParams => WarningKind::TooManyParams,
            LimitKey::FnCyclomatic | LimitKey::FnCognitive => WarningKind::ComplexFunction,
        }
    }
}
