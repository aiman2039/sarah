//! Internal module dependency graph: cycles, layers, fan-in/out, orphan files and
//! transitive context size. The import edges are resolved once and shared by every check.

mod context;
mod cycles;
mod fan;
mod layers;
mod orphans;
mod resolve;
mod rules;

pub use rules::{GraphRules, GraphRulesInput};

use self::resolve::ImportGraph;
use crate::config::{LimitRules, Limits};
use crate::count::FileStats;
use crate::report::Warning;

/// Stateless import-graph checks.
pub struct GraphHelpers;

/// What `GraphHelpers::check` needs: the files, their limits and the graph rules.
pub struct GraphCheck<'a> {
    pub files: &'a [FileStats],
    pub limits: &'a LimitRules,
    pub no_cycles: bool,
    pub rules: &'a GraphRules,
}

impl GraphHelpers {
    /// Every enabled graph rule; the edges are built only when a rule needs them.
    pub fn check(input: GraphCheck<'_>) -> Vec<Warning> {
        let GraphCheck {
            files,
            limits,
            no_cycles,
            rules,
        } = input;
        if !no_cycles && !rules.is_active() && !limits.sets_graph_limits() {
            return Vec::new();
        }
        let per_file = Self::graph_limits(files, limits);
        let graph = ImportGraph::build(files);
        [
            Self::check_cycles(files, &graph, no_cycles),
            Self::check_layers(files, &graph, rules),
            Self::check_fan(files, &graph, &per_file),
            Self::check_orphans(files, &graph, rules),
            Self::check_context(files, &graph, &per_file),
        ]
        .into_iter()
        .flatten()
        .collect()
    }

    /// The limits of each file; empty when no rule sets a graph limit.
    fn graph_limits(files: &[FileStats], limits: &LimitRules) -> Vec<Limits> {
        if !limits.sets_graph_limits() {
            return Vec::new();
        }
        files.iter().map(|f| limits.for_path(&f.path)).collect()
    }
}
