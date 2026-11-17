#!/usr/bin/env bash
# Set every "version" in the plugin manifests and server.json to the root
# Cargo.toml [package] version, whatever they hold now.
#   scripts/sync-versions.sh          rewrite them (bump-version.sh runs this)
#   scripts/sync-versions.sh --check  exit 1 when any differs (check.sh and Release run this)
set -euo pipefail
cd "$(dirname -- "${BASH_SOURCE[0]}")/.."

die() { echo "error: $*" >&2; exit 1; }

# Only the first match: the [package] version precedes every dependency table.
version="$(sed -n 's/^version = "\(.*\)"$/\1/p' Cargo.toml | head -n1)"
[[ "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+ ]] || die "No [package] version in Cargo.toml"
files=(plugins/sarah/.claude-plugin/plugin.json plugins/sarah/plugin.json server.json)

case "${1:-}" in
    --check)
        stale="$(grep -H '"version": "' "${files[@]}" | grep -vF "\"version\": \"$version\"" || true)"
        [[ -z "$stale" ]] || die "Versions differ from Cargo.toml ($version). Run scripts/sync-versions.sh:
$stale"
        ;;
    "")
        for f in "${files[@]}"; do
            sed -E "s/\"version\": \"[^\"]*\"/\"version\": \"$version\"/" "$f" > "$f.tmp" && mv "$f.tmp" "$f"
        done
        ;;
    *) echo "Usage: $0 [--check]" >&2; exit 2 ;;
esac
