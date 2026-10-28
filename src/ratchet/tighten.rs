use anyhow::{Context, Result};
use toml_edit::{DocumentMut, Item, TableLike, Value};

use super::maxima::Maxima;
use crate::config::LimitKey;

/// One lowered limit.
#[derive(Debug, PartialEq, Eq)]
pub struct Change {
    /// The rule's `path` glob.
    pub rule: String,
    pub key: LimitKey,
    pub from: usize,
    pub to: usize,
}

/// `sarah.toml` with lowered limits; comments and layout are kept.
pub struct Tightened {
    pub text: String,
    pub changes: Vec<Change>,
}

impl Maxima {
    /// Lower every `max_*` key of every `[[files]]` rule to the maximum it governs today.
    /// Keys a rule sets but no file reaches stay as they are.
    pub fn tighten(&self, config: &str) -> Result<Tightened> {
        let mut doc: DocumentMut = config.parse().context("cannot parse sarah.toml")?;
        let mut changes = Vec::new();
        for (index, rule) in Self::file_rules(&mut doc).into_iter().enumerate() {
            changes.extend(self.tighten_rule(index, rule)?);
        }
        Ok(Tightened {
            text: doc.to_string(),
            changes,
        })
    }

    /// `[[files]]` tables, or the inline tables of `files = [...]`, in order.
    fn file_rules(doc: &mut DocumentMut) -> Vec<&mut dyn TableLike> {
        match doc.get_mut("files") {
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

    fn tighten_rule(&self, index: usize, rule: &mut dyn TableLike) -> Result<Vec<Change>> {
        let path = rule
            .get("path")
            .and_then(Item::as_str)
            .unwrap_or_default()
            .to_string();
        let mut changes = Vec::new();
        for key in LimitKey::ALL {
            let (Some(value), Some(max)) = (rule.get_mut(key.name()), self.get(Some(index), key))
            else {
                continue;
            };
            let Some(from) = value.as_integer() else {
                continue;
            };
            let from = usize::try_from(from)
                .with_context(|| format!("{} in rule {path:?} is negative", key.name()))?;
            if max >= from {
                continue;
            }
            Self::set_keeping_comments(value, max)?;
            changes.push(Change {
                rule: path.clone(),
                key,
                from,
                to: max,
            });
        }
        Ok(changes)
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
            allows: Allows::default(),
        }
    }

    #[test]
    fn lowers_only_and_keeps_comments() {
        let text = "# top\n[[files]]\npath = \"**\"\nmax_file_lines = 300 # keep me\nmax_file_tokens = 5\n\n\
                    [[files]]\npath = \"tests/**\"\nmax_file_lines = 900\n";
        let config: Config = toml::from_str(text).unwrap();
        let rules = LimitRules::new(&config.files, LimitPatch::default()).unwrap();
        let files = [file("src/a.rs", 120), file("tests/t.rs", 400)];
        let tightened = Maxima::of(&files, &rules).tighten(text).unwrap();

        assert!(tightened.text.contains("max_file_lines = 120 # keep me"));
        assert!(tightened.text.contains("max_file_tokens = 5\n"));
        assert!(tightened.text.contains("max_file_lines = 400\n"));
        assert!(tightened.text.starts_with("# top\n"));
        assert_eq!(
            tightened.changes,
            vec![
                Change {
                    rule: "**".into(),
                    key: LimitKey::FileLines,
                    from: 300,
                    to: 120
                },
                Change {
                    rule: "tests/**".into(),
                    key: LimitKey::FileLines,
                    from: 900,
                    to: 400
                },
            ]
        );
    }
}
