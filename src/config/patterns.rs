use std::sync::Arc;

use anyhow::Result;
use regex::{Regex, RegexSet};

use crate::functions::InlineTestHelpers;
use crate::units::{LineNo, LineSpan};

/// Warnings kept per file and pattern; the last one counts the rest.
const MAX_HITS_PER_PATTERN: usize = 20;
/// Characters of the matching line kept in a warning.
const MAX_LINE_CHARS: usize = 120;

/// The `forbid_patterns` regexes of one `[[files]]` rule, tested against each line.
#[derive(Debug)]
pub struct LinePatterns {
    sources: Vec<String>,
    /// `sources` joined, computed once (for cache keys).
    key: Arc<str>,
    set: RegexSet,
}

/// One line that matched one pattern.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PatternHit {
    pub line: LineNo,
    pub pattern: String,
    /// The trimmed line, cut to `MAX_LINE_CHARS` characters.
    pub text: String,
    /// Matches of the same pattern after this one that were not kept (only on the last kept hit).
    pub more: usize,
}

impl LinePatterns {
    pub fn new(sources: &[String]) -> Result<Self> {
        let set = RegexSet::new(sources).map_err(|err| Self::first_invalid(sources, err))?;
        Ok(Self {
            sources: sources.to_vec(),
            key: sources.join("\u{0}").into(),
            set,
        })
    }

    pub fn is_empty(&self) -> bool {
        self.sources.is_empty()
    }

    /// Identifies the pattern list (for cache keys).
    pub fn key(&self) -> Arc<str> {
        Arc::clone(&self.key)
    }

    /// Matches per line, skipping lines in `skip` (stripped inline tests).
    pub fn scan(&self, text: &str, skip: &[LineSpan]) -> Vec<PatternHit> {
        let mut counts = vec![0usize; self.sources.len()];
        let mut hits = Vec::new();
        for (idx, raw) in text.lines().enumerate() {
            let line = LineNo(1).below(idx);
            if InlineTestHelpers::in_spans(line, skip) {
                continue;
            }
            for pat in &self.set.matches(raw) {
                let (Some(count), Some(pattern)) = (counts.get_mut(pat), self.sources.get(pat))
                else {
                    continue;
                };
                *count = count.saturating_add(1);
                if *count <= MAX_HITS_PER_PATTERN {
                    hits.push(PatternHit {
                        line,
                        pattern: pattern.clone(),
                        text: Self::shorten(raw.trim()),
                        more: 0,
                    });
                }
            }
        }
        Self::mark_overflow(&mut hits, &counts, &self.sources);
        hits
    }

    /// The error of the first pattern that does not compile; `set_err` when each compiles alone.
    fn first_invalid(sources: &[String], set_err: regex::Error) -> anyhow::Error {
        sources
            .iter()
            .find_map(|source| {
                let err = Regex::new(source).err()?;
                Some(anyhow::Error::new(err).context(format!("invalid regex {source:?}")))
            })
            .unwrap_or_else(|| anyhow::Error::new(set_err).context("invalid forbid_patterns"))
    }

    fn mark_overflow(hits: &mut [PatternHit], counts: &[usize], sources: &[String]) {
        for (source, count) in sources.iter().zip(counts) {
            let more = count.saturating_sub(MAX_HITS_PER_PATTERN);
            if more == 0 {
                continue;
            }
            if let Some(last) = hits.iter_mut().rev().find(|h| &h.pattern == source) {
                last.more = more;
            }
        }
    }

    fn shorten(line: &str) -> String {
        if line.chars().count() <= MAX_LINE_CHARS {
            return line.to_string();
        }
        let mut cut: String = line.chars().take(MAX_LINE_CHARS).collect();
        cut.push('…');
        cut
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn patterns(list: &[&str]) -> LinePatterns {
        LinePatterns::new(&list.iter().map(ToString::to_string).collect::<Vec<_>>()).unwrap()
    }

    #[test]
    fn matches_each_line_and_skips_spans() {
        let hits = patterns(&["unwrap\\(\\)", "^use "]).scan(
            "use a;\nlet x = y.unwrap();\n  z.unwrap()\n",
            &[LineSpan {
                start: LineNo(3),
                end: LineNo(3),
            }],
        );
        let got: Vec<_> = hits
            .iter()
            .map(|h| (h.line.0, h.pattern.as_str()))
            .collect();
        assert_eq!(got, vec![(1, "^use "), (2, "unwrap\\(\\)")]);
        assert_eq!(hits[1].text, "let x = y.unwrap();");
    }

    #[test]
    fn caps_hits_and_counts_the_rest() {
        let text = "x\n".repeat(25);
        let hits = patterns(&["x"]).scan(&text, &[]);
        assert_eq!(hits.len(), 20);
        assert_eq!(hits[19].more, 5);
        assert_eq!(hits[18].more, 0);
    }

    #[test]
    fn shortens_on_char_boundaries_and_rejects_bad_regex() {
        let long = "é".repeat(200);
        let hits = patterns(&["é"]).scan(&long, &[]);
        assert_eq!(hits[0].text.chars().count(), 121);
        let err = LinePatterns::new(&["(".to_string()]).unwrap_err();
        assert!(format!("{err:#}").contains("invalid regex \"(\""));
    }
}
