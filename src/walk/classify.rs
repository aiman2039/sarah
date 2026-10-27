use std::path::{Path, PathBuf};

use ignore::overrides::Override;

use super::skipped::Skipped;

pub(super) enum Visit {
    File(PathBuf),
    Skip(fn(&mut Skipped) -> &mut usize),
    Nothing,
}

impl Visit {
    /// Classify one walk result; walk errors (permissions, races) count as unreadable.
    pub(super) fn of(
        entry: Result<ignore::DirEntry, ignore::Error>,
        root: &Path,
        overrides: &Override,
    ) -> Self {
        match entry {
            Ok(entry) => Self::classify(&entry, root, overrides),
            Err(_) => Self::Skip(|s| &mut s.errors),
        }
    }

    fn classify(entry: &ignore::DirEntry, root: &Path, overrides: &Override) -> Self {
        let Some(file_type) = entry.file_type() else {
            return Self::Nothing;
        };
        if file_type.is_file() {
            // The root is canonical and directory links are never followed, so a
            // regular file's walk path has no symlink components: it is its realpath.
            return Self::File(entry.path().to_path_buf());
        }
        if !file_type.is_symlink() {
            return Self::Nothing;
        }
        let Ok(real) = dunce::canonicalize(entry.path()) else {
            return Self::Skip(|s| &mut s.broken_links);
        };
        if !real.starts_with(root) {
            return Self::Skip(|s| &mut s.outside_root);
        }
        if real.is_dir() {
            return Self::Skip(|s| &mut s.dir_links);
        }
        // The link itself passed the filters; its target must too.
        if overrides.matched(&real, false).is_ignore() {
            return Self::Skip(|s| &mut s.duplicates);
        }
        Self::File(real)
    }
}
