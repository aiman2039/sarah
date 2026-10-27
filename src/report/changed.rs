use std::collections::HashSet;
use std::path::{Path, PathBuf};

use super::model::{Report, Warning};

impl Report {
    /// Keep only warnings about changed files or the folders that hold them.
    pub fn retain_changed(&mut self, changed: &HashSet<PathBuf>) {
        self.warnings.retain(|w| Self::touches(w, changed));
    }

    fn touches(warning: &Warning, changed: &HashSet<PathBuf>) -> bool {
        if warning.path == Path::new(".") {
            return !changed.is_empty();
        }
        changed.iter().any(|path| path.starts_with(&warning.path))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::report::WarningKind;

    #[test]
    fn files_and_their_folders() {
        let changed: HashSet<PathBuf> = ["src/a/b.rs".into()].into();
        let touches = |p: &str| {
            Report::touches(
                &Warning::new(p.into(), WarningKind::LargeFile, String::new()),
                &changed,
            )
        };
        assert!(touches("src/a/b.rs"));
        assert!(touches("src/a"));
        assert!(touches("."));
        assert!(!touches("src/a/c.rs"));
        assert!(!touches("src/ab"));
    }
}
