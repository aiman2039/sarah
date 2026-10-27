use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};

/// Stateless helpers around the `git` CLI.
pub struct GitHelpers;

impl GitHelpers {
    /// Files changed since the merge base of `rev` and `HEAD`, including uncommitted and
    /// untracked files, relative to `root`.
    pub fn changed_since(root: &Path, rev: &str) -> Result<HashSet<PathBuf>> {
        let base = Self::git(root, &["merge-base", rev, "HEAD"])?;
        let base = base.trim();
        let tracked = Self::git(root, &["diff", "--name-only", "-z", "--relative", base])?;
        let untracked = Self::git(root, &["ls-files", "-z", "--others", "--exclude-standard"])?;
        Ok(tracked
            .split('\0')
            .chain(untracked.split('\0'))
            .filter(|p| !p.is_empty())
            .map(PathBuf::from)
            .collect())
    }

    fn git(root: &Path, args: &[&str]) -> Result<String> {
        let out = Command::new("git")
            .arg("-C")
            .arg(root)
            .args(args)
            .output()
            .context("cannot run git (needed by --changed-since)")?;
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
