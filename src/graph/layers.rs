use super::resolve::ImportGraph;
use super::{GraphHelpers, GraphRules};
use crate::count::FileStats;
use crate::report::{Warning, WarningKind};

/// The files, their edges and the layer of each file (`None`: in no layer).
struct LayerCheck<'a> {
    files: &'a [FileStats],
    graph: &'a ImportGraph<'a>,
    rules: &'a GraphRules,
    layers: Vec<Option<usize>>,
}

impl GraphHelpers {
    /// One `layer_violation` per edge from a lower layer to a higher one.
    pub(super) fn check_layers(
        files: &[FileStats],
        graph: &ImportGraph<'_>,
        rules: &GraphRules,
    ) -> Vec<Warning> {
        if !rules.has_layers() {
            return Vec::new();
        }
        let check = LayerCheck {
            files,
            graph,
            rules,
            layers: files.iter().map(|f| rules.layer_of(&f.path)).collect(),
        };
        (0..files.len())
            .flat_map(|from| check.violations(from))
            .collect()
    }
}

impl LayerCheck<'_> {
    fn layer(&self, idx: usize) -> Option<usize> {
        self.layers.get(idx).copied().flatten()
    }

    fn violations(&self, from: usize) -> Vec<Warning> {
        let Some(low) = self.layer(from) else {
            return Vec::new();
        };
        self.graph
            .targets(from)
            .filter(|&to| self.layer(to).is_some_and(|high| high > low))
            .filter_map(|to| self.warning(from, to))
            .collect()
    }

    fn warning(&self, from: usize, to: usize) -> Option<Warning> {
        let low = self.layer(from)?;
        let high = self.layer(to)?;
        let source = self.files.get(from)?;
        let target = self.files.get(to)?.path.display().to_string();
        let message = format!(
            "imports {target} of layer {high} {:?} from layer {low} {:?} — a layer may only import its own or lower layers",
            self.rules.layer_glob(high),
            self.rules.layer_glob(low),
        );
        let warning =
            Warning::new(source.path.clone(), WarningKind::LayerViolation, message).about(target);
        Some(match self.graph.line(from, to) {
            Some(line) => warning.at_line(line),
            None => warning,
        })
    }
}
