use serde::Serialize;

/// Entries the walk saw but did not count, so nothing disappears silently.
#[derive(Debug, Default, Serialize)]
pub struct Skipped {
    /// Symlinks whose target does not exist.
    pub broken_links: usize,
    /// Symlinks resolving outside the root.
    pub outside_root: usize,
    /// Directory symlinks. Never descended: an in-root target is walked by its
    /// real path anyway, so following them could only double count or loop.
    pub dir_links: usize,
    /// Symlinks to a file already counted, or whose target is excluded.
    pub duplicates: usize,
    /// Unreadable entries (permissions, races with deletion).
    pub errors: usize,
}

impl Skipped {
    pub fn total(&self) -> usize {
        [
            self.broken_links,
            self.outside_root,
            self.dir_links,
            self.duplicates,
            self.errors,
        ]
        .iter()
        .fold(0, |a, b| a.saturating_add(*b))
    }
}
