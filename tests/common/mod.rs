//! Helpers shared by the integration tests: temp fixtures and JSON runs of `sarah`.

use std::path::{Path, PathBuf};

use assert_cmd::Command;

/// A fresh, empty temp folder for one test.
#[expect(
    clippy::unwrap_used,
    reason = "test helper; a failure should abort the test"
)]
pub fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sarah-it-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Write each `(relative path, text)` below `dir`, creating folders as needed.
#[expect(
    clippy::unwrap_used,
    reason = "test helper; a failure should abort the test"
)]
pub fn write(dir: &Path, files: &[(&str, &str)]) {
    for (path, text) in files {
        let full = dir.join(path);
        std::fs::create_dir_all(full.parent().unwrap()).unwrap();
        std::fs::write(full, text).unwrap();
    }
}

/// The JSON report of `sarah <dir> -f json <args>`; the run must succeed.
#[expect(
    clippy::unwrap_used,
    reason = "test helper; a failure should abort the test"
)]
pub fn run_json(dir: &Path, args: &[&str]) -> serde_json::Value {
    let out = Command::cargo_bin("sarah")
        .unwrap()
        .arg(dir)
        .arg("-f")
        .arg("json")
        .args(args)
        .output()
        .unwrap();
    assert!(out.status.success(), "{out:?}");
    serde_json::from_slice(&out.stdout).unwrap()
}
