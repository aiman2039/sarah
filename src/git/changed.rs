use std::collections::HashSet;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};

/// Stateless helpers around the `git` CLI.
pub struct GitHelpers;

impl GitHelpers {
    pub fn merge_base(root: &Path, rev: &str) -> Result<String> {
        let base = Self::git(root, &["merge-base", rev, "HEAD"])?;
        Ok(base.trim().to_string())
    }

    /// Files changed since the merge base of `rev` and `HEAD`, including uncommitted,
    /// deleted and untracked files, relative to `root`.
    pub fn changed_since(root: &Path, rev: &str) -> Result<HashSet<PathBuf>> {
        let base = Self::merge_base(root, rev)?;
        Self::changed_from_base(root, &base)
    }

    pub fn changed_from_base(root: &Path, base: &str) -> Result<HashSet<PathBuf>> {
        let tracked = Self::git(root, &["diff", "--name-only", "-z", "--relative", base])?;
        let untracked = Self::git(root, &["ls-files", "-z", "--others", "--exclude-standard"])?;
        Ok(tracked
            .split('\0')
            .chain(untracked.split('\0'))
            .filter(|p| !p.is_empty())
            .map(PathBuf::from)
            .collect())
    }

    /// UTF-8 contents of `rel` (relative to `root`) at `base`, or `None` when absent/binary.
    pub fn file_at_base(root: &Path, base: &str, rel: &Path) -> Option<String> {
        let slash: Vec<_> = rel
            .components()
            .filter(|c| !matches!(c, Component::CurDir))
            .map(|c| c.as_os_str().to_string_lossy().into_owned())
            .collect();
        if slash.is_empty() {
            return None;
        }
        let spec = format!("{base}:./{}", slash.join("/"));
        let out = Command::new("git")
            .arg("-C")
            .arg(root)
            .args(["show", &spec])
            .output()
            .ok()?;
        if !out.status.success() {
            return None;
        }
        String::from_utf8(out.stdout).ok()
    }

    fn git(root: &Path, args: &[&str]) -> Result<String> {
        let out = Command::new("git")
            .arg("-C")
            .arg(root)
            .args(args)
            .output()
            .context("cannot run git")?;
        if !out.status.success() {
            bail!(
                "git {} failed: {}",
                args.join(" "),
                String::from_utf8_lossy(&out.stderr).trim()
            );
        }
        String::from_utf8(out.stdout).context("git printed a non-UTF-8 path")
    }
}
