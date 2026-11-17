# sarah in AI agents

All of these run `sarah mcp` (see the README's MCP section) on your machine.

## Claude Code plugin

The plugin gives the MCP server and a skill that tells Claude when to use it.

```sh
/plugin install sarah --marketplace aiman2039/sarah
```

When `sarah` on `PATH` is not the plugin's version, the plugin downloads that release
once, checks it against `SHA256SUMS.txt` and caches it. It needs `bash` and `curl`.

## Codex plugin

The same plugin folder, with an Agent Plugins manifest. Add the marketplace, then enable
`sarah` in `/plugins`:

```sh
codex plugin marketplace add aiman2039/sarah
```

## Claude Desktop

Download `sarah-<tag>-<target>.mcpb` from the
[latest release](https://github.com/aiman2039/sarah/releases/latest) and open it.
The server does not start in your project, so give the tools absolute paths.

## Any MCP client

With `sarah` installed (`cargo install sarah`, or a release archive):

```sh
claude mcp add sarah -- sarah mcp
codex mcp add sarah -- sarah mcp
```

sarah is in the [MCP Registry](https://registry.modelcontextprotocol.io) as
`io.github.aiman2039/sarah` (a `cargo` package).

## Files

| File | For |
|---|---|
| `.claude-plugin/marketplace.json` | Claude Code marketplace |
| `.agents/plugins/marketplace.json` | Codex marketplace |
| `plugins/sarah/.claude-plugin/plugin.json`, `.mcp.json` | Claude Code plugin |
| `plugins/sarah/plugin.json`, `mcp.json` | Codex plugin (Agent Plugins 1.0.0) |
| `plugins/sarah/scripts/sarah.sh` | Launcher: finds or downloads the matching binary |
| `plugins/sarah/skills/sarah/SKILL.md` | Skill, shared by both plugins |
| `mcpb/manifest.json` | Claude Desktop bundle; the Release workflow packs one per target |
| `server.json` | MCP Registry entry; the Release workflow publishes it |

`scripts/bump-version.sh` updates the plugin and registry versions; the Release workflow
fails if they differ from `Cargo.toml`.
