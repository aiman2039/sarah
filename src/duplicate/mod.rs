//! Cross-file and intra-file copy-paste detection over normalized code lines.

mod window;

use std::collections::{BTreeMap, HashMap};

use self::window::{WindowHelpers, WindowPos};
use crate::count::FileStats;
use crate::report::{Warning, WarningKind};
use crate::units::{LineNo, Lines};

struct CloneHit {
    other_file: usize,
    other_start: usize,
    other_end: usize,
    end_line: usize,
    matched: usize,
}

/// Stateless duplicate-block detector.
pub struct DuplicateHelpers;

impl DuplicateHelpers {
    pub fn check(files: &[FileStats], max_dup: Option<Lines>) -> Vec<Warning> {
        let Some(limit) = max_dup.filter(|l| l.0 > 0) else {
            return Vec::new();
        };
        let min_len = limit.0.saturating_add(1);
        let norms: Vec<&[(usize, String)]> =
            files.iter().map(|f| f.norm_lines.as_slice()).collect();
        let buckets = Self::build_buckets(&norms, min_len);
        let best = Self::collect_maximal(&norms, &buckets, min_len);
        Self::into_warnings(files, limit, best)
    }

    fn build_buckets(norms: &[&[(usize, String)]], min_len: usize) -> HashMap<u64, Vec<WindowPos>> {
        let mut buckets: HashMap<u64, Vec<WindowPos>> = HashMap::new();
        for (file_idx, lines) in norms.iter().enumerate() {
            if lines.len() < min_len {
                continue;
            }
            let end = lines.len().saturating_sub(min_len);
            for offset in 0..=end {
                if let Some(win) = lines.get(offset..offset.saturating_add(min_len)) {
                    let h = WindowHelpers::hash_slice(win);
                    buckets.entry(h).or_default().push(WindowPos {
                        file: file_idx,
                        offset,
                    });
                }
            }
        }
        buckets
    }

    fn collect_maximal(
        norms: &[&[(usize, String)]],
        buckets: &HashMap<u64, Vec<WindowPos>>,
        min_len: usize,
    ) -> BTreeMap<(usize, usize), CloneHit> {
        let mut best: BTreeMap<(usize, usize), CloneHit> = BTreeMap::new();
        for group in buckets.values().filter(|g| g.len() >= 2) {
            for (key, hit) in Self::scan_group(norms, group, min_len) {
                if best.get(&key).is_none_or(|prev| hit.matched > prev.matched) {
                    best.insert(key, hit);
                }
            }
        }
        best
    }

    fn scan_group(
        norms: &[&[(usize, String)]],
        group: &[WindowPos],
        min_len: usize,
    ) -> Vec<((usize, usize), CloneHit)> {
        let mut hits = Vec::new();
        for (idx, &a) in group.iter().enumerate() {
            for &b in group.get(idx.saturating_add(1)..).unwrap_or_default() {
                if let Some(hit) = Self::try_pair(norms, (a, b), min_len) {
                    hits.push(hit);
                }
            }
        }
        hits
    }

    fn try_pair(
        norms: &[&[(usize, String)]],
        pair: (WindowPos, WindowPos),
        min_len: usize,
    ) -> Option<((usize, usize), CloneHit)> {
        let (a, b) = pair;
        if a.file == b.file && a.offset.saturating_add(min_len) > b.offset {
            return None;
        }
        if a.offset > 0
            && b.offset > 0
            && WindowHelpers::line_eq(
                norms,
                WindowPos {
                    file: a.file,
                    offset: a.offset.saturating_sub(1),
                },
                WindowPos {
                    file: b.file,
                    offset: b.offset.saturating_sub(1),
                },
            )
        {
            return None;
        }
        let matched = Self::extend_len(norms, a, b);
        if matched < min_len {
            return None;
        }
        Self::build_hit(norms, (a, b), matched)
    }

    fn extend_len(norms: &[&[(usize, String)]], a: WindowPos, b: WindowPos) -> usize {
        let mut len = 0usize;
        loop {
            if a.file == b.file && a.offset.saturating_add(len) >= b.offset {
                break;
            }
            let pa = WindowPos {
                file: a.file,
                offset: a.offset.saturating_add(len),
            };
            let pb = WindowPos {
                file: b.file,
                offset: b.offset.saturating_add(len),
            };
            if !WindowHelpers::line_eq(norms, pa, pb) {
                break;
            }
            len = len.saturating_add(1);
        }
        len
    }

    fn build_hit(
        norms: &[&[(usize, String)]],
        pair: (WindowPos, WindowPos),
        matched: usize,
    ) -> Option<((usize, usize), CloneHit)> {
        let (a, b) = pair;
        let last = matched.saturating_sub(1);
        let a_start = norms.get(a.file)?.get(a.offset)?.0;
        let a_end = norms.get(a.file)?.get(a.offset.saturating_add(last))?.0;
        let b_start = norms.get(b.file)?.get(b.offset)?.0;
        let b_end = norms.get(b.file)?.get(b.offset.saturating_add(last))?.0;
        Some((
            (b.file, b_start),
            CloneHit {
                other_file: a.file,
                other_start: a_start,
                other_end: a_end,
                end_line: b_end,
                matched,
            },
        ))
    }

    fn into_warnings(
        files: &[FileStats],
        limit: Lines,
        best: BTreeMap<(usize, usize), CloneHit>,
    ) -> Vec<Warning> {
        best.into_iter()
            .filter_map(|((file_idx, start_line), hit)| {
                let path = &files.get(file_idx)?.path;
                let other = &files.get(hit.other_file)?.path;
                let other_loc = format!("{}:L{}-{}", other.display(), hit.other_start, hit.other_end);
                Some(
                    Warning::new(
                        path.clone(),
                        WarningKind::DuplicateCode,
                        format!(
                            "{} duplicate lines at L{}-{} (with {other_loc}, >{limit}) — extract shared helper",
                            hit.matched, start_line, hit.end_line
                        ),
                    )
                    .at_line(LineNo(start_line))
                    .about(other_loc),
                )
            })
            .collect()
    }
}
