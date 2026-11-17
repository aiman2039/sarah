use crate::count::FileStats;
use crate::report::warning::{Warning, WarningKind};

impl Warning {
    /// Lines matching `forbid_patterns`, found during analysis.
    pub fn for_patterns(files: &[FileStats]) -> Vec<Self> {
        files
            .iter()
            .flat_map(|f| {
                f.pattern_hits.iter().map(|hit| {
                    let more = if hit.more > 0 {
                        format!(" (+{} more matches of this pattern)", hit.more)
                    } else {
                        String::new()
                    };
                    Self::file(
                        f,
                        WarningKind::ForbiddenPattern,
                        format!(
                            "line L{} matches forbidden pattern {:?}: {}{more}",
                            hit.line, hit.pattern, hit.text
                        ),
                    )
                    .at_line(hit.line)
                    .about(hit.pattern.clone())
                })
            })
            .collect()
    }
}
