use std::collections::VecDeque;

use super::GraphHelpers;
use super::resolve::ImportGraph;
use crate::count::FileStats;
use crate::report::{Warning, WarningKind};

struct Tarjan {
    index: usize,
    indices: Vec<Option<usize>>,
    lowlink: Vec<usize>,
    stack: Vec<usize>,
    on_stack: Vec<bool>,
    sccs: Vec<Vec<usize>>,
}

impl GraphHelpers {
    /// One `circular_import` warning per strongly connected component of two or more files.
    /// Off unless `enabled` (`no_cycles`).
    pub(super) fn check_cycles(
        files: &[FileStats],
        graph: &ImportGraph<'_>,
        enabled: bool,
    ) -> Vec<Warning> {
        if !enabled || files.len() < 2 {
            return Vec::new();
        }
        let sccs = Tarjan::run(graph);
        sccs.into_iter()
            .filter(|scc| scc.len() >= 2)
            .filter_map(|scc| Self::cycle_warning(files, graph, &scc))
            .collect()
    }

    fn cycle_warning(
        files: &[FileStats],
        graph: &ImportGraph<'_>,
        scc: &[usize],
    ) -> Option<Warning> {
        let mut members = scc.to_vec();
        members.sort_by(|&a, &b| {
            files
                .get(a)
                .map(|f| &f.path)
                .cmp(&files.get(b).map(|f| &f.path))
        });
        let &start = members.first()?;
        let ring = Self::shortest_cycle(start, graph, scc)?;
        let names: Vec<String> = ring
            .iter()
            .filter_map(|&idx| Some(files.get(idx)?.path.display().to_string()))
            .collect();
        let path = files.get(start)?.path.clone();
        let chain = names.join(" -> ");
        Some(
            Warning::new(
                path,
                WarningKind::CircularImport,
                format!("circular import: {chain} — break the dependency cycle"),
            )
            .about(chain),
        )
    }

    fn shortest_cycle(start: usize, graph: &ImportGraph<'_>, scc: &[usize]) -> Option<Vec<usize>> {
        let mut prev: Vec<Option<usize>> = vec![None; graph.len()];
        let mut queue = VecDeque::new();
        for next in graph.targets(start) {
            if scc.contains(&next) {
                if let Some(p) = prev.get_mut(next) {
                    *p = Some(start);
                }
                queue.push_back(next);
            }
        }
        let last = Self::bfs_back_to(start, graph, (scc, &mut prev, &mut queue))?;
        Some(Self::reconstruct_ring(start, last, &prev))
    }

    fn bfs_back_to(
        start: usize,
        graph: &ImportGraph<'_>,
        state: (&[usize], &mut [Option<usize>], &mut VecDeque<usize>),
    ) -> Option<usize> {
        let (scc, prev, queue) = state;
        while let Some(curr) = queue.pop_front() {
            if graph.has_edge(curr, start) {
                return Some(curr);
            }
            Self::enqueue_neighbors((curr, graph), (scc, prev, queue));
        }
        None
    }

    fn enqueue_neighbors(
        from: (usize, &ImportGraph<'_>),
        state: (&[usize], &mut [Option<usize>], &mut VecDeque<usize>),
    ) {
        let (curr, graph) = from;
        let (scc, prev, queue) = state;
        for next in graph.targets(curr) {
            if !scc.contains(&next) || prev.get(next).copied().flatten().is_some() {
                continue;
            }
            if let Some(slot) = prev.get_mut(next) {
                *slot = Some(curr);
            }
            queue.push_back(next);
        }
    }

    fn reconstruct_ring(start: usize, last: usize, prev: &[Option<usize>]) -> Vec<usize> {
        let mut ring = vec![start, last];
        let mut cur = last;
        while cur != start {
            let Some(p) = prev.get(cur).copied().flatten() else {
                break;
            };
            if p == start {
                break;
            }
            ring.push(p);
            cur = p;
        }
        ring.reverse();
        ring.push(start);
        ring
    }
}

impl Tarjan {
    fn run(graph: &ImportGraph<'_>) -> Vec<Vec<usize>> {
        let n = graph.len();
        let mut state = Self {
            index: 0,
            indices: vec![None; n],
            lowlink: vec![0; n],
            stack: Vec::new(),
            on_stack: vec![false; n],
            sccs: Vec::new(),
        };
        for v in 0..n {
            if state.indices.get(v).copied().flatten().is_none() {
                state.strong_connect(v, graph);
            }
        }
        state.sccs
    }

    fn strong_connect(&mut self, v: usize, graph: &ImportGraph<'_>) {
        if let Some(slot) = self.indices.get_mut(v) {
            *slot = Some(self.index);
        }
        if let Some(low) = self.lowlink.get_mut(v) {
            *low = self.index;
        }
        self.index = self.index.saturating_add(1);
        self.stack.push(v);
        if let Some(on) = self.on_stack.get_mut(v) {
            *on = true;
        }
        for w in graph.targets(v) {
            self.visit_edge(v, w, graph);
        }
        if self.indices.get(v).copied().flatten() == self.lowlink.get(v).copied() {
            self.pop_scc(v);
        }
    }

    fn visit_edge(&mut self, v: usize, w: usize, graph: &ImportGraph<'_>) {
        if self.indices.get(w).copied().flatten().is_none() {
            self.strong_connect(w, graph);
            let w_low = self.lowlink.get(w).copied().unwrap_or(0);
            if let Some(v_low) = self.lowlink.get_mut(v) {
                *v_low = (*v_low).min(w_low);
            }
        } else if self.on_stack.get(w).copied().unwrap_or(false) {
            let w_idx = self.indices.get(w).copied().flatten().unwrap_or(0);
            if let Some(v_low) = self.lowlink.get_mut(v) {
                *v_low = (*v_low).min(w_idx);
            }
        }
    }

    fn pop_scc(&mut self, root: usize) {
        let mut scc = Vec::new();
        while let Some(w) = self.stack.pop() {
            if let Some(on) = self.on_stack.get_mut(w) {
                *on = false;
            }
            scc.push(w);
            if w == root {
                break;
            }
        }
        self.sccs.push(scc);
    }
}
