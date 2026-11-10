use anyhow::{Context, Result};
use toml_edit::{DocumentMut, Item, TableLike, Value};

use super::maxima::Maxima;
use crate::config::LimitKey;
use crate::structure::FolderLimitKey;

/// One lowered limit in `[[files]]` or `[[folder]]`.
#[derive(Debug, PartialEq, Eq)]
pub struct Change {
    pub section: &'static str,
    /// The rule's `path` glob.
    pub rule: String,
    pub key: &'static str,
    pub from: usize,
    pub to: usize,
}

/// `sarah.toml` with lowered limits; comments and layout are kept.
pub struct Tightened {
    pub text: String,
    pub changes: Vec<Change>,
}

struct LowerKey {
    section: &'static str,
    name: &'static str,
    max: usize,
}

impl Maxima {
    /// Lower every `max_*` key of every `[[files]]` and `[[folder]]` rule to the maximum
    /// it governs today. Keys a rule sets but no path reaches stay as they are.
    pub fn tighten(&self, config: &str) -> Result<Tightened> {
        let mut doc: DocumentMut = config.parse().context("cannot parse sarah.toml")?;
        let mut changes = Vec::new();
        for (index, rule) in Self::tables(&mut doc, "files").into_iter().enumerate() {
            for key in LimitKey::ALL {
                if let Some(max) = self.get_file(Some(index), key) {
                    let lk = LowerKey {
                        section: "files",
                        name: key.name(),
                        max,
                    };
                    changes.extend(Self::lower_one(rule, lk)?);
                }
            }
        }
        for (index, rule) in Self::tables(&mut doc, "folder").into_iter().enumerate() {
            for key in FolderLimitKey::ALL {
                if let Some(max) = self.get_folder(Some(index), key) {
                    let lk = LowerKey {
                        section: "folder",
                        name: key.name(),
                        max,
                    };
                    changes.extend(Self::lower_one(rule, lk)?);
                }
            }
        }
        Ok(Tightened {
            text: doc.to_string(),
            changes,
        })
    }

    fn tables<'a>(doc: &'a mut DocumentMut, section: &str) -> Vec<&'a mut dyn TableLike> {
        match doc.get_mut(section) {
            Some(Item::ArrayOfTables(tables)) => {
                tables.iter_mut().map(|t| t as &mut dyn TableLike).collect()
            }
            Some(Item::Value(Value::Array(array))) => array
                .iter_mut()
                .filter_map(Value::as_inline_table_mut)
                .map(|t| t as &mut dyn TableLike)
                .collect(),
            _ => Vec::new(),
        }
    }

    fn lower_one(rule: &mut dyn TableLike, lk: LowerKey) -> Result<Option<Change>> {
        let LowerKey { section, name, max } = lk;
        let path = rule
            .get("path")
            .and_then(Item::as_str)
            .unwrap_or_default()
            .to_string();
        let Some(value) = rule.get_mut(name) else {
            return Ok(None);
        };
        let Some(from) = value.as_integer() else {
            return Ok(None);
        };
        let from = usize::try_from(from)
            .with_context(|| format!("{name} in rule {path:?} is negative"))?;
        if max >= from {
            return Ok(None);
        }
        Self::set_keeping_comments(value, max)?;
        Ok(Some(Change {
            section,
            rule: path,
            key: name,
            from,
            to: max,
        }))
    }

    fn set_keeping_comments(item: &mut Item, to: usize) -> Result<()> {
        let Some(value) = item.as_value_mut() else {
            return Ok(());
        };
        let decor = value.decor().clone();
        *value = Value::from(i64::try_from(to).context("limit too large for TOML")?);
        *value.decor_mut() = decor;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::allow::Allows;
    use crate::config::{Config, LimitPatch, LimitRules};
    use crate::count::FileStats;
    use crate::structure::FolderRules;
    use crate::units::{Bytes, Lines, Tokens};

    fn file(path: &str, lines: usize) -> FileStats {
        FileStats {
            path: path.into(),
            language: "Rust".into(),
            bytes: Bytes(0),
            lines: Lines(lines),
            code: Lines(lines),
            comments: Lines(0),
            blanks: Lines(0),
            tokens: Tokens(lines),
            functions: None,
            types: Vec::new(),
            imports: Vec::new(),
            allows: Allows::default(),
        }
    }

    #[test]
    fn lowers_files_and_folders_and_keeps_comments() {
        let text = "# top\n[[files]]\npath = \"**\"\nmax_file_lines = 300 # keep me\nmax_file_tokens = 5\n\n\
                    [[folder]]\npath = \"src/**\"\nmax_files = 10\n";
        let config: Config = toml::from_str(text).unwrap();
        let file_rules = LimitRules::new(&config.files, LimitPatch::default()).unwrap();
        let folder_rules = FolderRules::new(config.preset, config.folder).unwrap();
        let files = [file("src/a/x.rs", 120), file("tests/t.rs", 200)];
        let tightened = Maxima::of(&files, &file_rules, &folder_rules)
            .tighten(text)
            .unwrap();

        assert!(tightened.text.contains("max_file_lines = 200 # keep me"));
        assert!(tightened.text.contains("max_file_tokens = 5\n"));
        assert!(tightened.text.contains("max_files = 1\n"));
        assert_eq!(tightened.changes.len(), 2);
    }
}
