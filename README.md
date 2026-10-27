# sarah

Counts tokens, lines and function sizes per file so AI agents (and humans) can see
*when*, *what* and *how* to refactor a repo.

Built on existing crates: `ignore` (gitignore-aware walking), `tiktoken-rs` (tokens),
`tokei` (code/comment/blank lines), `tree-sitter-tags` (function spans).

## Install

Download a binary for Linux (x86_64/aarch64, static musl), macOS (Apple Silicon/Intel)
or Windows (x86_64) from [Releases](https://github.com/aiman2039/sarah/releases), or:

```sh
cargo install --git https://github.com/aiman2039/sarah
```

## Usage

```sh
sarah [PATH]                      # human table
sarah -f json                     # full report for agents
sarah -f md                       # markdown to paste into an agent's context
sarah --fail-on-warn              # exit 1 if anything exceeds limits (CI / agent loops)
sarah -f md --json-out r.json     # markdown on stdout, full JSON in a file
```

Key flags: `--sort tokens|lines|name`, `-n/--top N`, `--tokenizer o200k|cl100k`,
`--max-file-tokens 8000`, `--max-file-lines 500`, `--max-fn-lines 80`,
`-i/--include GLOB`, `-e/--exclude GLOB`, `--hidden`, `--no-functions`,
`--no-default-excludes` (lockfiles, `*.min.js`, `*.map` are skipped by default).

Function extraction: Rust, Python, JavaScript, TypeScript/TSX, Go.

Token counts use OpenAI encodings; Claude's tokenizer isn't public, so treat them as a close approximation.

## GitHub Action

Installs the released binary for the runner (Linux, macOS, Windows; x64 and ARM64 where
released), writes the markdown report to the job summary and exposes the JSON report.

```yaml
- uses: actions/checkout@v6
- id: sarah
  uses: aiman2039/sarah@v0.2.0   # a release tag; @master installs the latest release
  with:
    path: .
    args: --max-fn-lines 60 --exclude 'vendor/**'
    fail-on-warn: true
- run: echo "${{ steps.sarah.outputs.tokens }} tokens, ${{ steps.sarah.outputs.warnings }} warnings"
```

| Input | Default | |
|---|---|---|
| `path` | `.` | Directory to analyze |
| `args` | | Extra `sarah` arguments |
| `fail-on-warn` | `false` | Fail the step when limits are exceeded |
| `summary` | `true` | Write the markdown report to the job summary |
| `version` | action ref if it is a tag, else latest | Release tag to install |
| `token` | `github.token` | Token used to download the release |

Outputs: `json` (report path), `tokens`, `warnings`.

## Development

Hooks install on the first `cargo test` (cargo-husky). `pre-commit` runs
`scripts/check.sh fmt` and `scripts/check.sh clippy`. Clippy runs pedantic plus the
safety lints in `[lints.clippy]` in `Cargo.toml`, with `-D warnings`.

```sh
sh scripts/check.sh          # fmt + clippy + tests + cargo-audit + cargo-deny (same as CI)
```

## Releasing

```sh
scripts/bump-version.sh               # 0.1.0 -> 0.1.1 in Cargo.toml + Cargo.lock (no commit)
git commit -am "Release v0.1.1" && git push
scripts/release.sh                    # trigger the Release workflow for origin/master's version

scripts/git-release.sh --check-only   # run every check, change nothing
scripts/git-release.sh                # all of the above in one step: check, bump, commit, tag, push, release
```

The Release workflow (`.github/workflows/release.yaml`) runs CI on macOS/Linux/Windows,
builds all five targets in parallel, uploads them to a draft release with
`SHA256SUMS.txt`, publishes it, then runs the action end to end against the new release.
CI runs only as part of Release.
