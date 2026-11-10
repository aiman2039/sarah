//! Integration tests for new rules, folder fixes, ratcheting, git diff and MCP check_file.

use std::path::{Path, PathBuf};
use std::process::Command as SysCommand;

use assert_cmd::Command;

#[expect(
    clippy::unwrap_used,
    reason = "test helper; a failure should abort the test"
)]
fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sarah-feat-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[expect(
    clippy::unwrap_used,
    reason = "test helper; a failure should abort the test"
)]
fn run_in(dir: &Path, args: &[&str]) -> (Option<i32>, String) {
    let out = Command::cargo_bin("sarah")
        .unwrap()
        .arg(dir)
        .args(args)
        .output()
        .unwrap();
    (out.status.code(), String::from_utf8(out.stdout).unwrap())
}

#[test]
fn new_file_fn_bool_type_import_and_unused_allow_rules() {
    let dir = temp_dir("rules");
    std::fs::write(
        dir.join("sarah.toml"),
        "unused_allow = true\n\
         [[files]]\n\
         path = \"**\"\n\
         max_file_fns = 1\n\
         max_fn_bool_params = 1\n\
         max_type_fields = 2\n\
         max_type_methods = 1\n\
         forbid_imports = [\"crate::cli*\"]\n",
    )
    .unwrap();
    let allow_marker = format!("// {}-allow(deep_function): unused", "sarah");
    std::fs::write(
        dir.join("lib.rs"),
        format!(
            "use crate::cli::Args;\n\n\
             pub struct God {{ pub a: u8, pub b: u8, pub c: u8 }}\n\
             impl God {{\n\
                 pub fn m1(&self, x: bool, y: bool) {{}}\n\
                 {allow_marker}\n\
                 pub fn m2(&self) {{}}\n\
             }}\n"
        ),
    )
    .unwrap();
    let (_, json) = run_in(&dir, &["-f", "json"]);
    std::fs::remove_dir_all(&dir).unwrap();
    for kind in [
        "\"too_many_functions\"",
        "\"too_many_bool_params\"",
        "\"large_type\"",
        "\"forbidden_import\"",
        "\"unused_allow\"",
    ] {
        assert!(json.contains(kind), "missing {kind} in:\n{json}");
    }
}

#[test]
fn split_hints_report_largest_and_deepest_blocks() {
    let dir = temp_dir("hints");
    std::fs::write(
        dir.join("a.rs"),
        "pub fn work(x: u8) -> u8 {\n\
             let mut out = 0;\n\
             if x > 0 {\n\
                 if x > 1 {\n\
                     out = x;\n\
                 }\n\
                 out += 1;\n\
                 out += 2;\n\
             }\n\
             out\n\
         }\n",
    )
    .unwrap();
    let (_, md) = run_in(
        &dir,
        &["-f", "md", "--max-fn-lines", "5", "--max-fn-depth", "1"],
    );
    std::fs::remove_dir_all(&dir).unwrap();
    assert!(md.contains("largest block at L3-9"), "{md}");
    assert!(md.contains("deepest at L4-6"), "{md}");
}

#[test]
fn ratchet_lowers_folders_and_check_exits_1() {
    let dir = temp_dir("ratchet-check");
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::write(dir.join("src/a.rs"), "fn a() {\n    1;\n    2;\n}\n").unwrap();
    std::fs::write(dir.join("src/b.rs"), "fn b() {}\n").unwrap();
    let run_sub = |sub: &[&str]| {
        let out = Command::cargo_bin("sarah")
            .unwrap()
            .args(sub)
            .arg(&dir)
            .output()
            .unwrap();
        (out.status.code(), String::from_utf8(out.stdout).unwrap())
    };
    assert_eq!(run_sub(&["init"]).0, Some(0));
    assert_eq!(run_sub(&["ratchet", "--check"]).0, Some(0));

    std::fs::remove_file(dir.join("src/b.rs")).unwrap();
    let (code, msg) = run_sub(&["ratchet", "--check"]);
    assert_eq!(code, Some(1), "{msg}");
    assert!(msg.contains("[[folder]] \"**\": max_files 2 -> 1"), "{msg}");

    assert_eq!(run_sub(&["ratchet"]).0, Some(0));
    assert_eq!(run_sub(&["ratchet", "--check"]).0, Some(0));
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn diff_reports_deltas_against_git_base() {
    let dir = temp_dir("diff");
    let git = |args: &[&str]| {
        let s = SysCommand::new("git")
            .arg("-C")
            .arg(&dir)
            .args(args)
            .status()
            .unwrap();
        assert!(s.success());
    };
    git(&["init", "-q"]);
    git(&["config", "user.email", "t@example.com"]);
    git(&["config", "user.name", "t"]);
    std::fs::write(dir.join("a.rs"), "pub fn a() {}\n").unwrap();
    git(&["add", "."]);
    git(&["commit", "-q", "-m", "init"]);

    std::fs::write(
        dir.join("a.rs"),
        "pub fn a() {\n    let x = 1;\n    let y = 2;\n    let _ = (x, y);\n}\n",
    )
    .unwrap();
    let (_, table) = run_in(&dir, &["--diff", "HEAD", "--max-fn-lines", "2"]);
    let (_, json) = run_in(
        &dir,
        &["-f", "json", "--diff", "HEAD", "--max-fn-lines", "2"],
    );
    std::fs::remove_dir_all(&dir).unwrap();
    assert!(
        table.contains("Diff vs HEAD: +0 files, +4 lines"),
        "{table}"
    );
    assert!(table.contains("+1 warnings"), "{table}");
    assert!(json.contains("\"warnings_delta\": 1"), "{json}");
}

#[test]
fn mcp_check_file_runs_direct_and_checks_parent_folder() {
    let dir = temp_dir("mcp-file");
    std::fs::write(dir.join("sarah.toml"), "preset = \"rust\"\n").unwrap();
    std::fs::create_dir_all(dir.join("src/topic")).unwrap();
    let bad = dir.join("src/topic/utils.rs");
    std::fs::write(&bad, "pub fn f() {}\n").unwrap();
    let req = format!(
        "{}\n{}\n{}\n",
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"t","version":"0"}}}"#,
        r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": {
                "name": "check_file",
                "arguments": {
                    "root": dir.to_string_lossy(),
                    "file": "src/topic/utils.rs"
                }
            }
        })
    );
    let out = Command::cargo_bin("sarah")
        .unwrap()
        .arg("mcp")
        .write_stdin(req)
        .output()
        .unwrap();
    std::fs::remove_dir_all(&dir).unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("folder_forbidden_name"), "{stdout}");
    assert!(stdout.contains("folder_missing_required"), "{stdout}");
}
