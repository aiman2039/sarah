//! End-to-end snapshots: report output is consumed by agents and the GitHub Action,
//! so any byte change must show up here.

use assert_cmd::Command;

const FIXTURE: &str = "tests/fixtures/repo";
const LIMITS: [&str; 6] = [
    "--max-file-tokens",
    "150",
    "--max-file-lines",
    "20",
    "--max-fn-lines",
    "10",
];

const RULES: &str = "tests/fixtures/rules.toml";

fn run(format: &str) -> String {
    run_with(&["-f", format])
}

#[expect(
    clippy::unwrap_used,
    reason = "test helper; a failure should abort the test"
)]
fn run_with(args: &[&str]) -> String {
    let out = sarah().args(args).output().unwrap();
    assert!(out.status.success(), "{out:?}");
    String::from_utf8(out.stdout).unwrap()
}

/// `sarah` on the fixture with low limits.
#[expect(
    clippy::unwrap_used,
    reason = "test helper; a failure should abort the test"
)]
fn sarah() -> Command {
    let mut cmd = Command::cargo_bin("sarah").unwrap();
    cmd.arg(FIXTURE).args(LIMITS);
    cmd
}

/// Windows prints paths with `\` (escaped as `\\` in JSON). SARIF embeds sarah's own
/// version, which every release bumps.
fn snapshot(name: &str, output: &str) {
    let version = format!(
        r#""version": "{}""#,
        env!("CARGO_PKG_VERSION").replace('.', r"\.")
    );
    let filters = vec![
        (r"\\\\", "/"),
        (r"\\", "/"),
        (version.as_str(), r#""version": "[sarah-version]""#),
    ];
    insta::with_settings!({ filters => filters }, {
        insta::assert_snapshot!(name, output);
    });
}

#[test]
fn table_report() {
    snapshot("table", &run("table"));
}

#[test]
fn markdown_report() {
    snapshot("markdown", &run("md"));
}

#[test]
fn json_report() {
    snapshot("json", &run("json"));
}

#[test]
fn fail_on_warn_exits_1() {
    sarah().arg("--fail-on-warn").assert().code(1);
}

#[test]
fn errors_exit_2() {
    Command::cargo_bin("sarah")
        .unwrap()
        .arg("tests/fixtures/missing")
        .assert()
        .code(2);
    sarah()
        .args(["--config", "tests/fixtures/missing.toml"])
        .assert()
        .code(2);
}

#[test]
fn folder_rules_and_dirs() {
    snapshot(
        "rules",
        &run_with(&["-f", "md", "--dirs", "--config", RULES]),
    );
}

#[test]
fn github_annotations() {
    snapshot("github", &run_with(&["-f", "github", "--config", RULES]));
}

#[test]
fn sarif_report() {
    snapshot("sarif", &run_with(&["-f", "sarif", "--config", RULES]));
}

#[test]
fn baseline_hides_known_warnings() {
    let baseline = std::env::temp_dir().join(format!("sarah-baseline-{}.json", std::process::id()));
    sarah()
        .args(["--config", RULES, "--json-out"])
        .arg(&baseline)
        .assert()
        .success();
    let out = sarah()
        .args(["--config", RULES, "--fail-on-warn", "--baseline"])
        .arg(&baseline)
        .output()
        .unwrap();
    std::fs::remove_file(&baseline).unwrap();
    assert!(out.status.success(), "{out:?}");
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("Warnings: none"), "{stdout}");
    assert!(
        stdout.contains("known warnings hidden by --baseline"),
        "{stdout}"
    );
}

#[test]
fn changed_since_runs() {
    sarah().args(["--changed-since", "HEAD"]).assert().success();
}

#[test]
fn allow_comments_hide_warnings_unless_no_allow() {
    // The malformed directive is written at run time, so sarah's own report stays clean.
    let dir = std::env::temp_dir().join(format!("sarah-allow-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::copy("tests/fixtures/allow/lib.rs", dir.join("lib.rs")).unwrap();
    std::fs::write(
        dir.join("bad.rs"),
        "// sarah-allow(large_function)
pub fn short() {}
",
    )
    .unwrap();
    let run = |extra: &[&str]| {
        let out = Command::cargo_bin("sarah")
            .unwrap()
            .arg(&dir)
            .args(["--max-fn-lines", "5", "--max-file-lines", "5"])
            .args(extra)
            .output()
            .unwrap();
        assert!(out.status.success(), "{out:?}");
        String::from_utf8(out.stdout).unwrap()
    };
    let shown = run(&[]);
    assert!(shown.contains("function reported"), "{shown}");
    assert!(!shown.contains("function allowed"), "{shown}");
    assert!(
        !shown.contains("lines (>5) — consider splitting"),
        "{shown}"
    );
    assert!(
        shown.contains("bad.rs: sarah-allow at L1: missing `: <reason>`"),
        "{shown}"
    );
    assert!(
        shown.contains("2 warnings allowed by sarah-allow comments"),
        "{shown}"
    );

    let all = run(&["--no-allow"]);
    std::fs::remove_dir_all(&dir).unwrap();
    assert!(all.contains("function allowed"), "{all}");
    assert!(all.contains("consider splitting"), "{all}");
    assert!(!all.contains("allowed by sarah-allow"), "{all}");
}
