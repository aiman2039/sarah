# More rules

All rules on this page are off by default. Each file-scope kind can be turned off with
`sarah-allow-file(kind): reason`. In `[[files]]`, the last matching rule that sets a key
wins; an empty list turns a list rule off.

```toml
layers = ["src/units/**", "src/config/**", "src/cli/**"]   # lowest layer first
orphan_files = true
entry_points = ["src/plugins/*.rs"]          # added to the built-in entry points
test_paths = ["tests/**", "**/*_test.*"]     # replaces the default test globs

[[files]]
path = "src/**/*.rs"
max_fan_in = 20
max_fan_out = 10
max_context_tokens = 40000
allow_imports = ["crate::units*", "std::*"]
max_fn_returns = 4
max_file_types = 5
max_line_length = 120
min_comment_ratio = 2
max_comment_ratio = 40
max_public_items = 15
require_test = ["tests/{stem}.rs", "{dir}/{stem}_test.rs"]
forbid_patterns = ['\.unwrap\(\)', 'dbg!']

[[folder]]
path = "src/**"
allow_imports = ["crate::*", "std::*"]
min_test_ratio = 50
max_test_ratio = 300
```

## Import graph

These rules use the resolved internal import edges (the same edges as `--no-cycles`).

| Key | CLI flag | Kind | Meaning |
|---|---|---|---|
| `layers` (top level) | | `layer_violation` | A file belongs to the first glob that matches it. A file must not import a file in a higher layer. Files in no layer are ignored. One warning for each bad import, at the import line. |
| `max_fan_in` | `--max-file-fan-in N` | `high_fan_in` | The number of files that import the file. |
| `max_fan_out` | `--max-file-fan-out N` | `high_fan_out` | The number of internal files that the file imports. |
| `orphan_files` (top level) | `--orphans` | `orphan_file` | No file imports the file, and it is not an entry point. |
| `allow_imports` | | `disallowed_import` | Each import must match one pattern (same syntax as `forbid_imports`). External imports such as `std::*` must be in the list too. If a file rule and a folder rule apply, the import must pass the two. |
| `max_context_tokens` | `--max-context-tokens N` | `large_context` | The tokens of the file plus the tokens of each file that it imports directly or transitively. Each file counts one time. |

Orphans: only Rust, Python, JS/JSX and TS/TSX files are checked. Built-in entry points:
the default test globs of `test_paths` (`tests/**`, `**/tests/**`, `**/test/**`,
`**/__tests__/**`, `**/*_test.*`, `**/test_*.py`, `**/*.test.*`, `**/*.spec.*`,
`**/*Test.java`, `**/*Tests.cs`), and `main.rs`, `lib.rs`, `build.rs`, `__init__.py`,
`__main__.py`, `main.py`, `manage.py`, `setup.py`, `conftest.py`, `index.*`,
`main.{js,jsx,ts,tsx}`, `*.config.*`, `*.d.ts`, and all files below `benches/`,
`examples/`, `bin/`, `fuzz/` and `scripts/`. A Rust file declared with `mod x;` is in use. A Python module is in use when the `__init__.py` of
its package is imported.

Limits of the graph: an import that the resolver cannot map to a file gives no edge (path
aliases such as `@/x`, Go packages, C include paths). `use crate::a::X` stops at
`a/mod.rs`, so fan-in collects on `mod.rs` files and `max_context_tokens` is too low for
Rust re-exports. A `#[path = ..] mod x;` is resolved as a plain `mod x;`.
The graph keys are not managed by `sarah init` and `sarah ratchet`.

## Functions, types and lines

| Key | CLI flag | Kind | Meaning |
|---|---|---|---|
| `max_fn_returns` | `--max-fn-returns N` | `too_many_returns` | Explicit `return` statements in a function. Nested functions and closures count for themselves. Tail expressions and Rust `?` do not count. JSON functions have a `returns` field. |
| `max_file_types` | `--max-file-types N` | `too_many_types` | The types that sarah extracts in the file (the types that `max_type_fields` checks). |
| `max_line_length` | `--max-line-length N` | `long_line` | Characters in a line. Blank lines and comment-only lines are ignored. One warning for each file, at the first long line. |
| `min_comment_ratio`, `max_comment_ratio` | `--min-comment-ratio N`, `--max-comment-ratio N` | `low_comment_ratio`, `high_comment_ratio` | Percent (0 to 100) of comment lines in code + comment lines. Files with no code lines are ignored. Not ratcheted. |
| `max_public_items` | `--max-file-public-items N` | `too_many_public_items` | Public functions, methods and types in the file. |

`too_many_returns` is a function kind: `sarah-allow(too_many_returns): reason` above the
function turns it off. `sarah init` writes the four `max_*` keys and `sarah ratchet`
lowers them.

Public items for each language:

- Rust: `pub` only, not `pub(crate)`, `pub(super)` or `pub(in ..)`.
- TypeScript and JavaScript: exported functions, classes, interfaces, type aliases and
  enums, and the non-private methods of an exported class. Re-exports do not count.
- Go: names that start with a capital letter.
- Python: module-level `def` and `class` with no `_` prefix. Methods do not count.
- Java and C#: `public` items. Kotlin and Scala: all items that are not `private`,
  `protected` or `internal`. Swift: `public` and `open`. PHP: functions, types, and
  methods that are not `private` or `protected`. Zig: `pub`.
- C, C++, Ruby, Lua, Elixir and Bash have no count and give no warning.

## Tests

| Key | Kind | Meaning |
|---|---|---|
| `require_test` (`[[files]]`) | `missing_test` | Path templates relative to the root. `{stem}` is the file name without the extension, `{name}` is the file name and `{dir}` is the folder of the file. The remaining text is a glob. A file passes when a template matches a different analyzed file, or when the file has an inline test (Rust `#[cfg(test)]`, Zig `test`). |
| `min_test_ratio`, `max_test_ratio` (`[[folder]]`) | `folder_low_test_ratio`, `folder_high_test_ratio` | Percent of test code lines to non-test code lines in the subtree. The value can be more than 100. Test code is the code of test files plus the inline tests of the other files. Only source files count. Not ratcheted. |
| `test_paths` (top level) | | The globs of test files. The default is `tests/**`, `**/tests/**`, `**/test/**`, `**/__tests__/**`, `**/*_test.*`, `**/test_*.py`, `**/*.test.*`, `**/*.spec.*`, `**/*Test.java`, `**/*Tests.cs`. |

`sarah lsp` and the MCP `check_file` tool see one file only, so they do not report
`missing_test`.

## Forbidden patterns

`forbid_patterns` (`[[files]]`) is a list of regexes (Rust `regex` syntax). Each line of
the file is tested. Each match gives one `forbidden_pattern` warning at that line; the
`subject` is the pattern. A file gets a maximum of 20 warnings for each pattern, and the
last warning shows how many more lines matched. With `exclude_inline_tests`, lines in
inline tests are ignored. An invalid regex is a config error.
