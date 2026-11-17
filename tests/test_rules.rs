//! Integration tests for `require_test` and `min_test_ratio`/`max_test_ratio`.

mod common;

use std::path::Path;

use assert_cmd::Command;

use common::{run_json, temp_dir, write};

/// `(path, kind)` of every warning of one of `kinds`, sorted.
fn warnings(report: &serde_json::Value, kinds: &[&str]) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = report["warnings"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|w| kinds.contains(&w["kind"].as_str().unwrap_or_default()))
        .map(|w| {
            (
                w["path"].as_str().unwrap_or_default().to_string(),
                w["kind"].as_str().unwrap_or_default().to_string(),
            )
        })
        .collect();
    out.sort();
    out
}

const INLINE: &str = "pub fn c() {}\n\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn t() {}\n}\n";

fn require_test_fixture(dir: &Path) {
    let allow = format!("// {}-allow-file(missing_test): thin wrapper\n", "sarah");
    write(
        dir,
        &[
            (
                "sarah.toml",
                "[[files]]\npath = \"src/**/*.rs\"\n\
                 require_test = [\"tests/{stem}.rs\", \"tests/**/{stem}_test.rs\", \"{dir}/{stem}_test.rs\"]\n\
                 [[files]]\npath = \"src/off/**\"\nrequire_test = []\n",
            ),
            ("src/a.rs", "pub fn a() {}\n"),
            ("src/b.rs", "pub fn b() {}\n"),
            ("tests/b.rs", "fn tb() {}\n"),
            ("src/deep/e.rs", "pub fn e() {}\n"),
            ("tests/x/y/e_test.rs", "fn te() {}\n"),
            ("src/f.rs", "pub fn f() {}\n"),
            ("src/f_test.rs", "fn tf() {}\n"),
            ("src/c.rs", INLINE),
            ("src/d.rs", &format!("{allow}pub fn d() {{}}\n")),
            ("src/off/g.rs", "pub fn g() {}\n"),
        ],
    );
}

#[test]
fn require_test_finds_test_files_and_inline_tests() {
    let dir = temp_dir("require");
    require_test_fixture(&dir);
    let plain = run_json(&dir, &[]);
    let stripped = run_json(&dir, &["--exclude-inline-tests"]);
    std::fs::remove_dir_all(&dir).unwrap();
    for report in [&plain, &stripped] {
        let got = warnings(report, &["missing_test"]);
        let want = vec![
            ("src/a.rs".to_string(), "missing_test".to_string()),
            // `src/f_test.rs` has no test of its own.
            ("src/f_test.rs".to_string(), "missing_test".to_string()),
        ];
        assert_eq!(got, want, "{report:#}");
    }
    assert_eq!(plain["allowed"], 1);
}

#[test]
fn invalid_require_test_template_is_a_config_error() {
    let dir = temp_dir("require-bad");
    write(
        &dir,
        &[
            (
                "sarah.toml",
                "[[files]]\npath = \"src/**\"\nrequire_test = [\"tests/[{stem}.rs\"]\n",
            ),
            ("src/a.rs", "pub fn a() {}\n"),
        ],
    );
    let out = Command::cargo_bin("sarah")
        .unwrap()
        .arg(&dir)
        .output()
        .unwrap();
    std::fs::remove_dir_all(&dir).unwrap();
    assert_eq!(out.status.code(), Some(2), "{out:?}");
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        err.contains("src/**") && err.contains("tests/[{stem}.rs"),
        "{err}"
    );
}

fn lines(prefix: &str, n: usize) -> String {
    (0..n)
        .map(|i| format!("pub fn {prefix}{i}() {{}}\n"))
        .collect::<Vec<_>>()
        .concat()
}

#[test]
fn test_ratio_counts_test_files_and_inline_tests_per_subtree() {
    let dir = temp_dir("ratio");
    let inline = format!(
        "{}#[cfg(test)]\nmod tests {{\n    #[test]\n    fn t() {{}}\n}}\n",
        lines("p", 5)
    );
    write(
        &dir,
        &[
            (
                "sarah.toml",
                "[[folder]]\npath = \"**\"\nmin_test_ratio = 50\n\
                 [[folder]]\npath = \"lib\"\nmax_test_ratio = 40\nmin_test_ratio = 0\n",
            ),
            // `app`: 10 non-test lines, 2 test lines -> 20% < 50%.
            ("app/main.rs", &lines("m", 10)),
            ("app/main_test.rs", &lines("t", 2)),
            // `lib`: 5 non-test lines, 5 inline test lines -> 100% > 40%.
            ("lib/core.rs", &inline),
            // `docs` has no non-test code: skipped.
            ("docs/tests/only.rs", &lines("d", 3)),
        ],
    );
    let report = run_json(&dir, &[]);
    let stripped = run_json(&dir, &["--exclude-inline-tests"]);
    std::fs::remove_dir_all(&dir).unwrap();
    let kinds = ["folder_low_test_ratio", "folder_high_test_ratio"];
    let want = vec![
        ("app".to_string(), "folder_low_test_ratio".to_string()),
        ("lib".to_string(), "folder_high_test_ratio".to_string()),
    ];
    assert_eq!(warnings(&report, &kinds), want, "{report:#}");
    assert_eq!(warnings(&stripped, &kinds), want, "{stripped:#}");
}

#[test]
fn test_paths_replaces_the_default_globs() {
    let dir = temp_dir("ratio-paths");
    write(
        &dir,
        &[
            (
                "sarah.toml",
                "test_paths = [\"**/checks/**\"]\n[[folder]]\npath = \".\"\nmin_test_ratio = 100\n",
            ),
            ("src/a.rs", &lines("a", 4)),
            ("checks/a.rs", &lines("c", 4)),
            ("tests/b.rs", &lines("b", 4)),
        ],
    );
    let report = run_json(&dir, &[]);
    std::fs::remove_dir_all(&dir).unwrap();
    // `tests/b.rs` is not a test file now and `sarah.toml` is not code: 4 / 8 = 50% < 100%.
    let w = &report["warnings"][0];
    assert_eq!(w["kind"], "folder_low_test_ratio", "{report:#}");
    assert!(w["message"].as_str().unwrap().contains("50%"), "{report:#}");
}
