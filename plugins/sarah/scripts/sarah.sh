#!/usr/bin/env bash
# Run the sarah release that matches this plugin's version.
# Uses `sarah` on PATH when it is that version. Else it downloads the release
# archive once, checks it against SHA256SUMS.txt and caches the binary.
# Stdout belongs to the command (the MCP stdio transport); messages go to stderr.
set -euo pipefail

repo="aiman2039/sarah"
root="$(cd "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
manifest="$root/.claude-plugin/plugin.json"

die() { echo "sarah plugin: $*" >&2; exit 1; }

version="$(grep -m1 '"version"' "$manifest" | sed 's/.*"version": *"\([^"]*\)".*/\1/')"
[[ "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || die "no version in $manifest"
tag="v$version"

if command -v sarah >/dev/null && [[ "$(sarah --version 2>/dev/null)" == "sarah $version" ]]; then
    exec sarah "$@"
fi

case "$(uname -s)-$(uname -m)" in
    Linux-x86_64 | Linux-amd64) target=x86_64-unknown-linux-musl ;;
    Linux-aarch64 | Linux-arm64) target=aarch64-unknown-linux-musl ;;
    Darwin-arm64) target=aarch64-apple-darwin ;;
    Darwin-x86_64) target=x86_64-apple-darwin ;;
    MINGW*-x86_64 | MSYS*-x86_64 | CYGWIN*-x86_64) target=x86_64-pc-windows-msvc ;;
    *) die "no sarah release for $(uname -s)-$(uname -m); install it with: cargo install sarah" ;;
esac
exe=sarah ext=tar.gz
if [[ "$target" == *windows* ]]; then exe=sarah.exe ext=zip; fi

# SARAH_PLUGIN_DATA: set by the Codex plugin; CLAUDE_PLUGIN_DATA: set by Claude Code.
data="${SARAH_PLUGIN_DATA:-${CLAUDE_PLUGIN_DATA:-${XDG_CACHE_HOME:-$HOME/.cache}/sarah}}"
cache="$data/bin/$tag"
bin="$cache/$exe"

if [[ ! -x "$bin" ]]; then
    command -v curl >/dev/null || die "curl is required to download sarah $tag"
    asset="sarah-$tag-$target.$ext"
    url="https://github.com/$repo/releases/download/$tag"
    mkdir -p "$cache"
    tmp="$(mktemp -d "$cache/.download.XXXXXX")"
    trap 'rm -rf "$tmp"' EXIT
    echo "sarah plugin: downloading $asset" >&2
    curl -fsSL --retry 3 -o "$tmp/$asset" "$url/$asset"
    curl -fsSL --retry 3 -o "$tmp/SHA256SUMS.txt" "$url/SHA256SUMS.txt"
    (
        cd "$tmp"
        grep " $asset\$" SHA256SUMS.txt > expected.txt
        if command -v sha256sum >/dev/null; then sha256sum -c expected.txt; else shasum -a 256 -c expected.txt; fi
        if [[ "$ext" == zip ]]; then
            if command -v unzip >/dev/null; then unzip -q "$asset"; else "$WINDIR/System32/tar.exe" -xf "$asset"; fi
        else
            tar -xzf "$asset"
        fi
    ) >&2 || die "checksum or extract failed for $asset"
    chmod +x "$tmp/$exe"
    mv -f "$tmp/$exe" "$bin"
fi

exec "$bin" "$@"
