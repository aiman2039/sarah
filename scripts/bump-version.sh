#!/usr/bin/env bash
# Bump the patch version (X.Y.Z -> X.Y.Z+1) in Cargo.toml and Cargo.lock.
# Prints the new version. Does not commit; see scripts/release.sh.
set -euo pipefail
cd "$(dirname -- "${BASH_SOURCE[0]}")/.."

die() { echo "error: $*" >&2; exit 1; }

old="$(sed -n 's/^version = "\(.*\)"$/\1/p' Cargo.toml | head -n1)"
[[ "$old" =~ ^([0-9]+)\.([0-9]+)\.([0-9]+)$ ]] || die "Expected a stable major.minor.patch version, got '$old'"
new="${BASH_REMATCH[1]}.${BASH_REMATCH[2]}.$((BASH_REMATCH[3] + 1))"

# Only the first match: the [package] version precedes every dependency table.
awk -v old="$old" -v new="$new" '
    !done && $0 == "version = \"" old "\"" { print "version = \"" new "\""; done = 1; next }
    { print }
' Cargo.toml > Cargo.toml.tmp && mv Cargo.toml.tmp Cargo.toml
CARGO_HUSKY_DONT_INSTALL_HOOKS=true cargo update --workspace --offline --quiet

echo "Bumped $old -> $new" >&2
echo "$new"
