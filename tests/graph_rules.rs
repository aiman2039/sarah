//! Integration tests for the import-graph rules: layers, fan-in/out, orphans,
//! allow_imports and max_context_tokens.

mod common;

use std::path::Path;

use assert_cmd::Command;
use common::{run_json, temp_dir, write};
use serde_json::Value;

/// `(path, kind, subject)` of every warning.
#[expect(
    clippy::unwrap_used,
    reason = "test helper; a failure should abort the test"
)]
fn warnings(dir: &Path, args: &[&str]) -> Vec<(String, String, String)> {
    let json = run_json(dir, args);
    json.get("warnings")
        .and_then(Value::as_array)
        .unwrap()
        .iter()
        .map(|w| {
            let text = |key: &str| {
                w.get(key)
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .replace('\\', "/")
            };
            (text("path"), text("kind"), text("subject"))
        })
        .collect()
}

fn sorted(mut v: Vec<(String, String)>) -> Vec<(String, String)> {
    v.sort();
    v
}

fn of_kind(all: &[(String, String, String)], kind: &str) -> Vec<(String, String)> {
    all.iter()
        .filter(|(_, k, _)| k == kind)
        .map(|(p, _, s)| (p.clone(), s.clone()))
        .collect()
}

const TS_APP: &[(&str, &str)] = &[
    (
        "index.ts",
        "import { a } from \"./core/a\";\nimport { u } from \"./ui/u\";\n",
    ),
    (
        "core/a.ts",
        "import { b } from \"./b\";\nimport { u } from \"../ui/u\";\nexport const a = 1;\n",
    ),
    ("core/b.ts", "export const b = 2;\n"),
    (
        "ui/u.ts",
        "import { b } from \"../core/b\";\nexport const u = 3;\n",
    ),
    ("ui/unused.ts", "export const z = 4;\n"),
    ("README.md", "# docs\n"),
];

#[test]
fn rules_are_off_by_default() {
    let dir = temp_dir("off");
    write(&dir, TS_APP);
    let all = warnings(&dir, &[]);
    std::fs::remove_dir_all(&dir).unwrap();
    assert!(all.is_empty(), "{all:?}");
}

#[test]
fn layers_flag_imports_of_higher_layers() {
    let dir = temp_dir("layers");
    write(&dir, TS_APP);
    write(
        &dir,
        &[("sarah.toml", "layers = [\"core/**\", \"ui/**\"]\n")],
    );
    let all = warnings(&dir, &[]);
    std::fs::remove_dir_all(&dir).unwrap();
    assert_eq!(
        of_kind(&all, "layer_violation"),
        vec![("core/a.ts".to_string(), "ui/u.ts".to_string())]
    );
}

#[test]
fn fan_in_fan_out_and_context_limits() {
    let dir = temp_dir("fan");
    write(&dir, TS_APP);
    let all = warnings(
        &dir,
        &[
            "--max-file-fan-in",
            "1",
            "--max-file-fan-out",
            "1",
            "--max-context-tokens",
            "40",
        ],
    );
    std::fs::remove_dir_all(&dir).unwrap();
    let paths = |kind| -> Vec<String> {
        sorted(of_kind(&all, kind))
            .into_iter()
            .map(|(p, _)| p)
            .collect()
    };
    assert_eq!(paths("high_fan_in"), vec!["core/b.ts", "ui/u.ts"]);
    assert_eq!(paths("high_fan_out"), vec!["core/a.ts", "index.ts"]);
    let context = paths("large_context");
    assert!(context.contains(&"index.ts".to_string()), "{all:?}");
    assert!(!context.contains(&"core/b.ts".to_string()), "{all:?}");
}

#[test]
fn files_keys_and_allow_file_comments() {
    let dir = temp_dir("files-keys");
    write(&dir, TS_APP);
    let allow = format!("// {}-allow-file(high_fan_in): shared constants\n", "sarah");
    write(
        &dir,
        &[
            ("core/b.ts", &format!("{allow}export const b = 2;\n")),
            (
                "sarah.toml",
                "[[files]]\npath = \"**\"\nmax_fan_in = 1\nmax_fan_out = 5\nmax_context_tokens = 100000\n",
            ),
        ],
    );
    let all = warnings(&dir, &[]);
    std::fs::remove_dir_all(&dir).unwrap();
    assert_eq!(
        of_kind(&all, "high_fan_in"),
        vec![("ui/u.ts".to_string(), String::new())]
    );
}

#[test]
fn orphans_skip_entry_points_mod_declarations_and_non_code() {
    let dir = temp_dir("orphans");
    write(&dir, TS_APP);
    write(
        &dir,
        &[
            ("src/lib.rs", "mod graph;\npub use graph::G;\n"),
            (
                "src/graph/mod.rs",
                "pub(crate) mod fan;\nmod r#type;\npub struct G;\n",
            ),
            ("src/graph/fan.rs", "pub fn f() {}\n"),
            ("src/graph/type.rs", "pub fn t() {}\n"),
            ("src/graph/stray.rs", "pub fn s() {}\n"),
            ("pkg/__init__.py", "from .a import x\n"),
            ("pkg/a.py", "x = 1\n"),
            ("app.py", "from pkg import b\n"),
            ("pkg/b.py", "y = 2\n"),
            ("lone.py", "z = 3\n"),
            ("tools/gen.go", "package main\n"),
        ],
    );
    let cli = warnings(&dir, &["--orphans"]);
    write(
        &dir,
        &[(
            "sarah.toml",
            "orphan_files = true\nentry_points = [\"app.py\", \"lone.py\"]\n",
        )],
    );
    let config = warnings(&dir, &[]);
    std::fs::remove_dir_all(&dir).unwrap();
    let paths = |all: &[(String, String, String)]| -> Vec<String> {
        sorted(of_kind(all, "orphan_file"))
            .into_iter()
            .map(|(p, _)| p)
            .collect()
    };
    assert_eq!(
        paths(&cli),
        vec!["app.py", "lone.py", "src/graph/stray.rs", "ui/unused.ts"]
    );
    assert_eq!(paths(&config), vec!["src/graph/stray.rs", "ui/unused.ts"]);
}

#[test]
fn allow_imports_for_files_and_folders() {
    let dir = temp_dir("allow-imports");
    write(&dir, TS_APP);
    write(
        &dir,
        &[(
            "sarah.toml",
            "[[files]]\npath = \"core/**\"\nallow_imports = [\"core/*\"]\n\
             [[folder]]\npath = \"ui\"\nallow_imports = [\"./x*\"]\n",
        )],
    );
    let all = warnings(&dir, &[]);
    std::fs::remove_dir_all(&dir).unwrap();
    assert_eq!(
        sorted(of_kind(&all, "disallowed_import")),
        vec![
            ("core/a.ts".to_string(), "../ui/u".to_string()),
            ("ui/u.ts".to_string(), "../core/b".to_string()),
        ]
    );
}

#[test]
fn bad_layer_glob_is_an_error() {
    let dir = temp_dir("bad-glob");
    write(
        &dir,
        &[
            ("a.ts", "export const a = 1;\n"),
            ("sarah.toml", "layers = [\"[\"]\n"),
        ],
    );
    let out = Command::cargo_bin("sarah")
        .unwrap()
        .arg(&dir)
        .output()
        .unwrap();
    std::fs::remove_dir_all(&dir).unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&out.stderr).contains("layers"));
}
