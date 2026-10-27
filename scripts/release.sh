#!/usr/bin/env bash
# Trigger the Release workflow for the version on origin/master. The workflow
# creates the v<version> tag, runs CI, builds every target and publishes.
#   scripts/bump-version.sh && git commit -am "Release vX.Y.Z" && git push
#   scripts/release.sh
set -euo pipefail
cd "$(dirname -- "${BASH_SOURCE[0]}")/.."

die() { echo "error: $*" >&2; exit 1; }

gh auth status >/dev/null 2>&1 || die "gh is not authenticated. Run: gh auth login"
git fetch --no-tags --quiet origin master

# Release what CI will check out: origin/master, so local state must match it.
version="$(git show refs/remotes/origin/master:Cargo.toml | sed -n 's/^version = "\(.*\)"$/\1/p' | head -n1)"
tag="v$version"
local_version="$(sed -n 's/^version = "\(.*\)"$/\1/p' Cargo.toml | head -n1)"
test "$local_version" = "$version" \
    || die "Local version $local_version is not on origin/master ($version). Commit and push the bump first."
test -z "$(git ls-remote --tags origin "refs/tags/$tag")" \
    || die "Tag $tag already exists on origin. Run scripts/bump-version.sh first."

gh workflow run release.yaml --ref master
echo "Started Release for $tag. Follow it with: gh run watch \$(gh run list --workflow release.yaml --limit 1 --json databaseId --jq '.[0].databaseId')"
