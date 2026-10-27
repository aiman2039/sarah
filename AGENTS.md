# AGENTS.md

Guidance for AI coding agents working in this repository.

## Generic
- Always be brief and concise. If the user needs more information, they will ask you to elaborate.
- Always use popular, well-maintained packages first. Don't reinvent the wheel.
- Always ask for specific permission, and show the change, before you change AGENTS.md or rust.md.
- Always respond in ASD-STE100 Simplified Technical English. Code, commands and technical terms are exempt. If STE is
  not possible or not expressive enough, tell the user.
- Don't write or run tests, lints and similar checks in the middle of an implementation. Finish it first, then run them
  at the end to verify. This avoids slow, repeated iterations. Exceptions: a fast compile check (`cargo build`) is
  permitted during the work, and verify after each step when the user asks for step-by-step verification.

## Rules

- Follow @rust.md for all Rust code. Clippy enforces most of it (`[lints.clippy]` in `Cargo.toml`); the rest
  (newtypes, input structs, `*Helpers` structs, flat orchestrators, one `mod.rs`-rooted directory per topic) is on you.
- Suppress a lint only with `#[expect(lint, reason = "...")]`. `#[expect]` on a macro statement is ignored; put it on a function.
- Stdout is written only by `cli/output.rs` (`CliOutput`). No `println!`/`eprintln!` anywhere else.
- Default branch is `master`.

## Checks

- `sh scripts/check.sh` runs fmt, clippy (`-D warnings`), tests, cargo-audit and cargo-deny. Run it before finishing:
  CI runs only inside the Release workflow, never on push or PR.
- The pre-commit hook (cargo-husky, installed by the first `cargo test`) runs fmt + clippy.

## Invariants

- Report output (table, markdown, JSON) is consumed by agents and the GitHub Action. Keep it byte-identical unless the
  change is intended; JSON newtypes stay `#[serde(transparent)]`.
- Token counts must equal `tiktoken`'s exactly. Large files are tokenized in chunks cut only at the boundaries in
  `count/chunk.rs`; changing that rule or the tokenizer needs an exactness test against unsplit counts.
- `vendor/bpe-openai` is a patched copy (see `vendor/bpe-openai/SARAH-PATCH.md`) wired in via `[patch.crates-io]`.
  Don't edit or remove it unless syncing with upstream. Exclude it when running sarah on itself: `-e 'vendor/**'`.
- Walk: files are keyed by realpath; directory symlinks are never followed; skipped entries are counted in `Skipped`.

## Releases

- `scripts/bump-version.sh` (patch bump, no commit) → commit + push → `scripts/release.sh` triggers the Release workflow.
  `scripts/git-release.sh` does all of it in one step.
- `action.yml` downloads `sarah-<tag>-<target>.{tar.gz,zip}` plus `SHA256SUMS.txt`; keep it in sync with
  `.github/actions/release-build/action.yml` and the build matrix in `release.yaml`.
- The Rust version is pinned in `rust-toolchain.toml` and repeated in the workflows (`RUST_VERSION`, `toolchain:`); bump all together.
