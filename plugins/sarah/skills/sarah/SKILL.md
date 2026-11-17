---
name: sarah
description: Find refactor candidates and keep code small with sarah's MCP tools (token, line and function-size metrics, limits from sarah.toml). Use before you plan a refactor, after you edit a file, and before you finish a branch.
when_to_use: The user asks what to refactor, which files or functions are too large, how many tokens a file or repo has, or to check the code against sarah.toml limits.
---

# sarah

sarah counts tokens (exact `tiktoken` counts), lines and function sizes per file. It flags
files, functions, types, imports, cycles, duplicates and folders that exceed limits.
Limits come from `sarah.toml` at the repository root, or from defaults.

Use the `sarah` MCP tools. Each tool takes `args`: extra CLI flags, for example
`["--max-fn-lines", "60"]`. Paths are relative to the server's working directory;
give an absolute path when you are not sure.

| Task | Tool |
|---|---|
| Get an overview of a repo, or pick what to refactor | `report` (markdown) |
| Get the warnings on the current branch only | `warnings` with `changed_since: "origin/master"` (or the default branch) |
| Check a file after you edit it | `check_file` with `file` |
| Show what a branch added or removed | `diff` with `rev` |
| Lower the limits in `sarah.toml` to today's maximums | `ratchet` (`dry_run: true` by default; set `false` only when the user asks) |

## Workflow

1. Before a refactor, run `report` and start with the largest warnings.
2. After you edit a file, run `check_file` on it. Fix new warnings before you continue.
3. Before you finish, run `warnings` with `changed_since` set to the default branch.
   Report the remaining warnings to the user.

Do not edit `sarah.toml` limits to make warnings go away, unless the user asks.
