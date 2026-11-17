use super::GraphHelpers;
use super::resolve::ImportGraph;
use crate::config::Limits;
use crate::count::FileStats;
use crate::report::{Warning, WarningKind};
use crate::units::Tokens;

/// Depth-first walk state, reused across start files: `seen[i] == stamp` marks file `i`
/// as visited in the current walk, so no per-file clearing is needed.
struct Closure {
    seen: Vec<usize>,
    stamp: usize,
    stack: Vec<usize>,
}

impl GraphHelpers {
    /// `large_context` for files whose transitive import closure exceeds `max_context_tokens`.
    pub(super) fn check_context(
        files: &[FileStats],
        graph: &ImportGraph<'_>,
        limits: &[Limits],
    ) -> Vec<Warning> {
        let mut closure = Closure {
            seen: vec![0; files.len()],
            stamp: 0,
            stack: Vec::new(),
        };
        files
            .iter()
            .zip(limits)
            .enumerate()
            .filter_map(|(idx, (file, limit))| {
                let max = limit.context_tokens?;
                let (total, count) = closure.tokens(files, graph, idx);
                (total > max).then(|| {
                    Warning::file(
                        file,
                        WarningKind::LargeContext,
                        format!(
                            "{total} tokens of context across {count} files (>{max}) — the file plus everything it imports transitively; cut imports or split the file"
                        ),
                    )
                })
            })
            .collect()
    }
}

impl Closure {
    /// Tokens and file count of `start` plus every file reachable from it; each file once.
    fn tokens(
        &mut self,
        files: &[FileStats],
        graph: &ImportGraph<'_>,
        start: usize,
    ) -> (Tokens, usize) {
        self.stamp = self.stamp.saturating_add(1);
        self.stack.push(start);
        let mut total = Tokens(0);
        let mut count = 0usize;
        while let Some(idx) = self.stack.pop() {
            match self.seen.get_mut(idx) {
                Some(slot) if *slot != self.stamp => *slot = self.stamp,
                _ => continue,
            }
            total = total.saturating_add(files.get(idx).map_or(Tokens(0), |f| f.tokens));
            count = count.saturating_add(1);
            self.stack.extend(graph.targets(idx));
        }
        (total, count)
    }
}
