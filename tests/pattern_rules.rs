//! Integration tests for `forbid_patterns`, in the report and in the LSP.

mod common;

use std::path::Path;

use assert_cmd::Command;

use common::{run_json, temp_dir, write};

/// 25 `.unwrap()` lines in `src/many.rs`, one `dbg!` each in `src/one.rs`, an allowed
/// file and a folder whose rule turns `forbid_patterns` off.
fn forbid_patterns_fixture(dir: &Path) {
    let many: String = (0..25)
        .map(|i| format!("let v{i} = x.unwrap();\n"))
        .collect::<Vec<_>>()
        .concat();
    let allow = format!("// {}-allow-file(forbidden_pattern): legacy\n", "sarah");
    write(
        dir,
        &[
            (
                "sarah.toml",
                "[[files]]\npath = \"**\"\nforbid_patterns = [\"\\\\.unwrap\\\\(\\\\)\", \"dbg!\"]\n\
                 [[files]]\npath = \"off/**\"\nforbid_patterns = []\n",
            ),
            ("src/many.rs", &many),
            ("src/one.rs", "fn f() {\n    dbg!(1);\n}\n"),
            ("src/allowed.rs", &format!("{allow}fn g() {{ dbg!(2); }}\n")),
            ("off/x.rs", "fn h() { dbg!(3); }\n"),
        ],
    );
}

#[test]
fn forbid_patterns_warn_per_line_with_a_cap() {
    let dir = temp_dir("patterns");
    forbid_patterns_fixture(&dir);
    let report = run_json(&dir, &[]);
    std::fs::remove_dir_all(&dir).unwrap();
    let all: Vec<&serde_json::Value> = report["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|w| w["kind"] == "forbidden_pattern")
        .collect();
    let many_hits: Vec<_> = all.iter().filter(|w| w["path"] == "src/many.rs").collect();
    assert_eq!(many_hits.len(), 20, "{report:#}");
    assert!(
        many_hits[19]["message"]
            .as_str()
            .unwrap()
            .contains("+5 more")
    );
    assert_eq!(many_hits[0]["line"], 1);
    assert_eq!(many_hits[0]["subject"], "\\.unwrap\\(\\)");
    let one: Vec<_> = all.iter().filter(|w| w["path"] == "src/one.rs").collect();
    assert_eq!(one.len(), 1);
    assert_eq!(one[0]["line"], 2);
    assert!(one[0]["message"].as_str().unwrap().contains("dbg!(1);"));
    assert!(
        !all.iter()
            .any(|w| w["path"] == "off/x.rs" || w["path"] == "src/allowed.rs")
    );
    assert_eq!(report["allowed"], 1);
}

#[test]
fn invalid_forbid_pattern_is_a_config_error() {
    let dir = temp_dir("patterns-bad");
    write(
        &dir,
        &[
            (
                "sarah.toml",
                "[[files]]\npath = \"src/**\"\nforbid_patterns = [\"(open\"]\n",
            ),
            ("src/a.rs", "fn a() {}\n"),
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
    assert!(err.contains("src/**") && err.contains("(open"), "{err}");
}

#[test]
fn lsp_applies_forbid_patterns_but_not_require_test() {
    let dir = temp_dir("lsp");
    let file = dir.join("big.rs");
    let text = "fn f() {\n    todo_later();\n}\n";
    std::fs::write(&file, text).unwrap();
    std::fs::write(
        dir.join("sarah.toml"),
        "[[files]]\npath = \"**\"\nforbid_patterns = [\"todo_later\"]\nrequire_test = [\"tests/{stem}.rs\"]\n",
    )
    .unwrap();
    let root_uri = format!("file://{}", dir.display());
    let file_uri = format!("file://{}", file.display());
    let frame = |val: serde_json::Value| {
        let s = serde_json::to_string(&val).unwrap();
        format!("Content-Length: {}\r\n\r\n{s}", s.len())
    };
    let input = [
        frame(serde_json::json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"rootUri":root_uri}})),
        frame(serde_json::json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":file_uri,"text":text}}})),
        frame(serde_json::json!({"jsonrpc":"2.0","id":2,"method":"shutdown"})),
        frame(serde_json::json!({"jsonrpc":"2.0","method":"exit"})),
    ]
    .concat();
    let out = Command::cargo_bin("sarah")
        .unwrap()
        .arg("lsp")
        .write_stdin(input)
        .output()
        .unwrap();
    std::fs::remove_dir_all(&dir).unwrap();
    assert!(out.status.success(), "{out:?}");
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("forbidden_pattern"), "{stdout}");
    assert!(!stdout.contains("missing_test"), "{stdout}");
}
