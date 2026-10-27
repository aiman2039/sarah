# sarah

Counts tokens, lines and function sizes per file so AI agents (and humans) can see
*when*, *what* and *how* to refactor a repo.

Built on existing crates: `ignore` (gitignore-aware walking), `tiktoken-rs` (tokens),
`tokei` (code/comment/blank lines), `tree-sitter-tags` (function spans).

## Usage

```sh
cargo install --path .
sarah [PATH]                      # human table
sarah -f json                     # full report for agents
sarah -f md                       # markdown to paste into an agent's context
sarah --fail-on-warn              # exit 1 if anything exceeds limits (CI / agent loops)
```

Key flags: `--sort tokens|lines|name`, `-n/--top N`, `--tokenizer o200k|cl100k`,
`--max-file-tokens 8000`, `--max-file-lines 500`, `--max-fn-lines 80`,
`-i/--include GLOB`, `-e/--exclude GLOB`, `--hidden`, `--no-functions`,
`--no-default-excludes` (lockfiles, `*.min.js`, `*.map` are skipped by default).

Function extraction: Rust, Python, JavaScript, TypeScript/TSX, Go.

Token counts use OpenAI encodings; Claude's tokenizer isn't public, so treat them as a close approximation.
