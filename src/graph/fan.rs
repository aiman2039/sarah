use super::GraphHelpers;
use super::resolve::ImportGraph;
use crate::config::Limits;
use crate::count::FileStats;
use crate::report::{Warning, WarningKind};
use crate::units::Fan;

impl GraphHelpers {
    /// `high_fan_in` / `high_fan_out` for files over `max_fan_in` / `max_fan_out`.
    pub(super) fn check_fan(
        files: &[FileStats],
        graph: &ImportGraph<'_>,
        limits: &[Limits],
    ) -> Vec<Warning> {
        let fan_in = Self::fan_in(graph);
        let mut out = Vec::new();
        for (idx, ((file, limit), incoming)) in files.iter().zip(limits).zip(&fan_in).enumerate() {
            if let Some(max) = limit.fan_in.filter(|max| *incoming > *max) {
                out.push(Warning::file(
                    file,
                    WarningKind::HighFanIn,
                    format!(
                        "imported by {incoming} files (>{max}) — a change here ripples widely; narrow its API or split it"
                    ),
                ));
            }
            let outgoing = Fan(graph.fan_out(idx));
            if let Some(max) = limit.fan_out.filter(|max| outgoing > *max) {
                out.push(Warning::file(
                    file,
                    WarningKind::HighFanOut,
                    format!(
                        "imports {outgoing} internal files (>{max}) — reduce coupling or split the file"
                    ),
                ));
            }
        }
        out
    }

    /// Distinct importers per file (edges hold no duplicates and no self-edges).
    fn fan_in(graph: &ImportGraph<'_>) -> Vec<Fan> {
        let mut counts = vec![Fan(0); graph.len()];
        for to in (0..graph.len()).flat_map(|from| graph.targets(from)) {
            if let Some(count) = counts.get_mut(to) {
                count.0 = count.0.saturating_add(1);
            }
        }
        counts
    }
}
