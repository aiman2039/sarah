#!/usr/bin/env bash
# Validate, bump the patch version, commit, tag, push, and start the Release
# workflow. Mirrors terminator's `cargo xtask release`.
#   scripts/git-release.sh               full release from master
#   scripts/git-release.sh --check-only  run the checks only; change nothing
set -euo pipefail
cd "$(dirname -- "${BASH_SOURCE[0]}")/.."

check_only=false
case "${1:-}" in
    --check-only) check_only=true ;;
    "") ;;
    *) echo "Usage: $0 [--check-only]" >&2; exit 2 ;;
esac

die() { echo "error: $*" >&2; exit 1; }

test -z "$(git status --porcelain)" || die "Working tree is not clean. Commit or stash first."
head="$(git rev-parse HEAD)"
branch="$(git symbolic-ref --short HEAD)"

if ! $check_only; then
    test "$branch" = master || die "Release must run from master. Use --check-only on other branches."
    gh auth status >/dev/null
    git fetch --no-tags origin master
    git merge-base --is-ancestor refs/remotes/origin/master "$head" \
        || die "origin/master has commits that are not in HEAD. Pull first."
fi

sh scripts/check.sh all

if $check_only; then
    echo "Checks passed. Version, commits and remote are unchanged."
    exit 0
fi

# Pre-compute the patch bump scripts/bump-version.sh makes, to check the tag first.
old="$(sed -n 's/^version = "\(.*\)"$/\1/p' Cargo.toml | head -n1)"
[[ "$old" =~ ^([0-9]+)\.([0-9]+)\.([0-9]+)$ ]] || die "Expected a stable major.minor.patch version, got '$old'"
tag="v${BASH_REMATCH[1]}.${BASH_REMATCH[2]}.$((BASH_REMATCH[3] + 1))"

test -z "$(git tag --list "$tag")" || die "Tag $tag already exists locally. No files changed."
test -z "$(git ls-remote --tags origin "refs/tags/$tag")" || die "Tag $tag already exists on origin. No files changed."

test "v$(bash scripts/bump-version.sh)" = "$tag" || die "Version bump did not produce $tag."
git add Cargo.toml Cargo.lock
test "$(git diff --cached --name-only | sort | tr '\n' ' ')" = "Cargo.lock Cargo.toml " \
    || die "Unexpected staged changes after the version bump."

# The tree was just validated; only the version changed, so skip the hook's re-run.
validated="$(git write-tree)"
SARAH_RELEASE_VALIDATED_TREE="$validated" git commit --quiet -m "Release $tag"
revision="$(git rev-parse HEAD)"
git tag "$tag" "$revision"
git push --atomic origin "$revision:refs/heads/master" "refs/tags/$tag"

gh workflow run release.yaml --ref master -f "tag=$tag" \
    || die "$tag was pushed, but workflow dispatch failed. Retry only: gh workflow run release.yaml --ref master -f tag=$tag"
echo "Started Release for $tag ($revision)."
