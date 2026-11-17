//! Integration tests for returns, file types, line length, comment ratio and public-item rules.

mod common;

use std::path::{Path, PathBuf};

use assert_cmd::Command;
use common::{run_json, write};
use serde_json::Value;

/// A temp folder holding `files`.
fn temp_dir(name: &str, files: &[(&str, &str)]) -> PathBuf {
    let dir = common::temp_dir(name);
    write(&dir, files);
    dir
}

#[expect(
    clippy::unwrap_used,
    reason = "test helper; a failure should abort the test"
)]
fn run(dir: &Path, args: &[&str]) -> (Option<i32>, String) {
    let out = Command::cargo_bin("sarah")
        .unwrap()
        .args(args)
        .arg(dir)
        .output()
        .unwrap();
    (out.status.code(), String::from_utf8(out.stdout).unwrap())
}

/// `path: message` (or `path: message up to " ("` when `head`) of each `kind` warning, sorted.
fn listed(json: &Value, kind: &str, head: bool) -> Vec<String> {
    let text = |v: &Value| v.as_str().unwrap_or_default().to_string();
    let mut out: Vec<String> = json["warnings"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|w| w["kind"] == kind)
        .map(|w| {
            let msg = text(&w["message"]);
            let msg = if head {
                msg.split(" (").next().unwrap_or_default().to_string()
            } else {
                msg
            };
            format!("{}: {msg}", text(&w["path"]))
        })
        .collect();
    out.sort();
    out
}

const RUST_RETURNS: &str = "pub fn f(x: u8) -> u8 {\n    if x == 0 {\n        return 0;\n    }\n    let g = |y: u8| {\n        return y;\n    };\n    if x == 1 {\n        return 1;\n    }\n    return g(x);\n}\n\npub fn h() {}\n";
const PY_RETURNS: &str = "def f(x):\n    if x == 0:\n        return 0\n    g = lambda: 1\n    if x == 1:\n        return 1\n    return 2\n\ndef h():\n    pass\n";
const TS_RETURNS: &str = "export function f(x: number): number {\n  if (x === 0) {\n    return 0;\n  }\n  const g = () => {\n    return 1;\n  };\n  if (x === 1) {\n    return 1;\n  }\n  return g();\n}\nexport function h() {}\n";
const GO_RETURNS: &str = "package p\n\nfunc F(x int) int {\n\tg := func() int { return 1 }\n\tif x == 0 {\n\t\treturn 0\n\t}\n\tif x == 1 {\n\t\treturn 1\n\t}\n\treturn g()\n}\n\nfunc H() {}\n";
const RETURNS_FILES: [(&str, &str); 4] = [
    ("a.rs", RUST_RETURNS),
    ("a.py", PY_RETURNS),
    ("a.ts", TS_RETURNS),
    ("a.go", GO_RETURNS),
];

#[test]
fn returns_are_counted_per_language_and_warned() {
    let dir = temp_dir("returns", &RETURNS_FILES);
    let json = run_json(&dir, &["--max-fn-returns", "2"]);
    let mut counts: Vec<String> = json["functions"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["returns"] != 0)
        .map(|f| format!("{} {} {}", f["path"], f["name"], f["returns"]))
        .collect();
    counts.sort();
    assert_eq!(
        counts,
        [
            r#""a.go" "F" 3"#,
            r#""a.py" "f" 3"#,
            r#""a.rs" "f" 3"#,
            r#""a.ts" "f" 3"#,
            r#""a.ts" "g" 1"#
        ]
    );
    assert_eq!(
        listed(&json, "too_many_returns", true),
        [
            "a.go: function F at L3-12: 3 return statements",
            "a.py: function f at L1-7: 3 return statements",
            "a.rs: function f at L1-12: 3 return statements",
            "a.ts: function f at L1-12: 3 return statements",
        ]
    );
    let quiet = run_json(&dir, &["--max-fn-returns", "3"]);
    assert!(listed(&quiet, "too_many_returns", true).is_empty());
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn returns_respect_function_and_file_allows() {
    let dir = temp_dir("returns-allow", &RETURNS_FILES);
    let allow = format!("// {}-allow(too_many_returns): early exits\n", "sarah");
    let file_allow = format!("# {}-allow-file(too_many_returns): generated\n", "sarah");
    write(
        &dir,
        &[
            ("a.rs", &format!("{allow}{RUST_RETURNS}")),
            ("a.py", &format!("{file_allow}{PY_RETURNS}")),
            (
                "sarah.toml",
                "[[files]]\npath = \"**\"\nmax_fn_returns = 2\n",
            ),
        ],
    );
    let found = listed(&run_json(&dir, &[]), "too_many_returns", true);
    assert_eq!(found.len(), 2, "{found:?}");
    assert!(found[0].starts_with("a.go") && found[1].starts_with("a.ts"));
    std::fs::remove_dir_all(&dir).unwrap();
}

const PUBLIC_FILES: [(&str, &str); 5] = [
    (
        "a.rs",
        "pub fn a() {}\npub(crate) fn b() {}\npub struct S;\nimpl S {\n    pub fn m(&self) {}\n    fn n(&self) {}\n}\n",
    ),
    (
        "a.py",
        "def a():\n    pass\n\ndef _b():\n    pass\n\nclass C:\n    def m(self):\n        pass\n",
    ),
    (
        "a.ts",
        "export function a() {}\nfunction b() {}\nexport class C {\n  m() {}\n  private n() {}\n}\n",
    ),
    (
        "a.go",
        "package p\n\nfunc A() {}\nfunc b() {}\n\ntype S struct{}\n\nfunc (s S) M() {}\n",
    ),
    ("a.c", "int a(void) {}\nint b(void) {}\n"),
];

#[test]
fn public_items_per_language() {
    let dir = temp_dir("public", &PUBLIC_FILES);
    let json = run_json(&dir, &["--max-file-public-items", "1"]);
    assert_eq!(
        listed(&json, "too_many_public_items", true),
        [
            "a.go: 3 public items",
            "a.py: 2 public items",
            "a.rs: 3 public items",
            "a.ts: 3 public items"
        ]
    );
    write(
        &dir,
        &[(
            "sarah.toml",
            "[[files]]\npath = \"*.rs\"\nmax_public_items = 2\n",
        )],
    );
    let json = run_json(&dir, &[]);
    assert_eq!(
        listed(&json, "too_many_public_items", true),
        ["a.rs: 3 public items"]
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn file_types_and_long_lines() {
    let long = "x".repeat(30);
    let src = format!(
        "struct A;\nstruct B;\nstruct C {{ x: u8 }}\nfn f() {{\n    let s = \"{long}\";\n    // {long}{long}\n    let é = \"ééééééééééé\";\n}}\n"
    );
    let dir = temp_dir("types-lines", &[("a.rs", &src), ("b.rs", "struct A;\n")]);
    let json = run_json(&dir, &["--max-file-types", "2", "--max-line-length", "25"]);
    assert_eq!(
        listed(&json, "too_many_types", false),
        ["a.rs: 3 types (>2) — move types into their own modules"]
    );
    assert_eq!(
        listed(&json, "long_line", false),
        ["a.rs: 2 lines longer than 25 chars (first at L5, longest 45)"]
    );
    let warnings = json["warnings"].as_array().unwrap();
    assert!(
        warnings
            .iter()
            .any(|w| w["kind"] == "long_line" && w["line"] == 5)
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn comment_ratio_bounds() {
    let dir = temp_dir(
        "ratio",
        &[
            ("dense.rs", "// a\n// b\n// c\nfn f() {}\n"),
            ("sparse.rs", "fn f() {}\nfn g() {}\nfn h() {}\n"),
            ("only.rs", "// nothing but comments\n"),
            ("notes.txt", "plain text\n"),
            (
                "sarah.toml",
                "[[files]]\npath = \"*.rs\"\nmin_comment_ratio = 10\nmax_comment_ratio = 50\n",
            ),
        ],
    );
    let json = run_json(&dir, &[]);
    assert_eq!(
        listed(&json, "high_comment_ratio", false),
        [
            "dense.rs: comment ratio 75% (>50%): 3 comment lines for 1 code lines — remove commented-out code and stale comments"
        ]
    );
    assert_eq!(
        listed(&json, "low_comment_ratio", true),
        ["sparse.rs: comment ratio 0%"]
    );
    let cli = run_json(
        &dir,
        &["--min-comment-ratio", "0", "--max-comment-ratio", "80"],
    );
    assert!(listed(&cli, "low_comment_ratio", true).is_empty());
    assert!(listed(&cli, "high_comment_ratio", true).is_empty());
    write(
        &dir,
        &[(
            "sarah.toml",
            "[[files]]\npath = \"**\"\nmax_comment_ratio = 101\n",
        )],
    );
    assert_eq!(run(&dir, &[]).0, Some(2));
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn rules_are_opt_in_and_init_writes_new_maxima() {
    let dir = temp_dir("init", &RETURNS_FILES);
    let json = run_json(&dir, &[]);
    for kind in [
        "too_many_returns",
        "too_many_types",
        "long_line",
        "low_comment_ratio",
        "high_comment_ratio",
        "too_many_public_items",
    ] {
        assert!(listed(&json, kind, true).is_empty(), "{kind}");
    }
    assert_eq!(run(&dir, &["init"]).0, Some(0));
    let config = std::fs::read_to_string(dir.join("sarah.toml")).unwrap();
    for line in [
        "max_fn_returns = 3\n",
        "max_file_types = 0\n",
        "max_public_items = 2\n",
        "max_line_length = 38\n",
    ] {
        assert!(config.contains(line), "{line}: {config}");
    }
    assert!(!config.contains("comment_ratio"), "{config}");
    std::fs::remove_dir_all(&dir).unwrap();
}
