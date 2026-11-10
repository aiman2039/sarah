//! Integration tests for cross-file rules, inline tests, folder CLI flags, LSP and MCP tools.

use std::path::{Path, PathBuf};

use assert_cmd::Command;

#[expect(
    clippy::unwrap_used,
    reason = "test helper; a failure should abort the test"
)]
fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sarah-w2-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[expect(
    clippy::unwrap_used,
    reason = "test helper; a failure should abort the test"
)]
fn run_json(dir: &Path, args: &[&str]) -> String {
    let out = Command::cargo_bin("sarah")
        .unwrap()
        .arg(dir)
        .arg("-f")
        .arg("json")
        .args(args)
        .output()
        .unwrap();
    assert!(out.status.success(), "{out:?}");
    String::from_utf8(out.stdout).unwrap()
}

#[test]
fn duplicates_cycles_imports_todos_and_empty_allow_reasons() {
    let dir = temp_dir("cross");
    let allow_empty = format!("// {}-allow(large_function):", "sarah");
    let todo_tag = format!("// {}: clean up", "TODO");
    let body =
        "    let alpha = 100;\n    let beta = 200;\n    let gamma = 300;\n    let delta = 400;\n";
    std::fs::write(
        dir.join("a.ts"),
        format!("import {{ b }} from \"./b\";\n{todo_tag}\n{allow_empty}\nexport function fa() {{\n{body}}}\n"),
    )
    .unwrap();
    std::fs::write(
        dir.join("b.ts"),
        format!("import {{ fa }} from \"./a\";\nexport function fb() {{\n{body}}}\n"),
    )
    .unwrap();
    let json = run_json(
        &dir,
        &[
            "--no-cycles",
            "--max-duplicate-lines",
            "3",
            "--max-file-imports",
            "0",
            "--max-todos",
            "0",
            "--require-allow-reason",
        ],
    );
    std::fs::remove_dir_all(&dir).unwrap();
    for kind in [
        "\"circular_import\"",
        "\"duplicate_code\"",
        "\"too_many_imports\"",
        "\"todo_comment\"",
        "\"bad_allow\"",
    ] {
        assert!(json.contains(kind), "missing {kind} in:\n{json}");
    }
}

#[test]
fn exclude_inline_tests_and_markdown_allows_ignored() {
    let dir = temp_dir("inline-tests");
    let md_allow = format!("# {}-allow-file(large_file): example", "sarah");
    std::fs::write(
        dir.join("README.md"),
        format!("```python\n{md_allow}\n```\nLine 4\nLine 5\nLine 6\n"),
    )
    .unwrap();
    std::fs::write(
        dir.join("lib.rs"),
        "pub fn prod() -> u8 {\n    1\n}\n\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn big_test() {\n        let a = 1;\n        let b = 2;\n        let c = 3;\n        assert_eq!(a + b, c);\n    }\n}\n",
    )
    .unwrap();
    let json = run_json(
        &dir,
        &[
            "--exclude-inline-tests",
            "--max-file-lines",
            "5",
            "--max-fn-lines",
            "4",
        ],
    );
    std::fs::remove_dir_all(&dir).unwrap();
    assert!(!json.contains("big_test"), "{json}");
    assert!(json.contains("\"path\": \"README.md\""), "{json}");
    assert!(!json.contains("\"allowed\""), "{json}");
}

#[test]
fn relative_forbid_imports_and_folder_cli_flags() {
    let dir = temp_dir("rel-folder");
    std::fs::create_dir_all(dir.join("src/domain/sub")).unwrap();
    std::fs::create_dir_all(dir.join("src/cli")).unwrap();
    std::fs::write(
        dir.join("sarah.toml"),
        "[[files]]\npath = \"src/domain/**\"\nforbid_imports = [\"src/cli*\"]\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("src/domain/sub/item.ts"),
        "import { run } from \"../../cli/app\";\nexport const x = 1;\n",
    )
    .unwrap();
    std::fs::write(dir.join("src/cli/app.ts"), "export const run = 1;\n").unwrap();
    let json = run_json(
        &dir,
        &[
            "--max-folder-files",
            "0",
            "--max-folder-total-files",
            "1",
            "--max-folder-depth",
            "2",
        ],
    );
    std::fs::remove_dir_all(&dir).unwrap();
    for kind in [
        "\"forbidden_import\"",
        "\"folder_too_many_files\"",
        "\"folder_too_many_total_files\"",
        "\"folder_too_deep\"",
    ] {
        assert!(json.contains(kind), "missing {kind} in:\n{json}");
    }
}

#[test]
fn lsp_publishes_diagnostics_over_stdio() {
    let dir = temp_dir("lsp");
    let file = dir.join("big.rs");
    std::fs::write(&file, "pub fn f(a: bool, b: bool) {}\n").unwrap();
    std::fs::write(
        dir.join("sarah.toml"),
        "[[files]]\npath = \"**\"\nmax_fn_bool_params = 1\n",
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
        frame(serde_json::json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":file_uri,"text":"pub fn f(a: bool, b: bool) {}\n"}}})),
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
    assert!(stdout.contains("publishDiagnostics"), "{stdout}");
    assert!(stdout.contains("too_many_bool_params"), "{stdout}");
}
