# Releasing

```sh
scripts/bump-version.sh               # 0.1.0 -> 0.1.1 in Cargo.toml + Cargo.lock (no commit)
git commit -am "Release v0.1.1" && git push
scripts/release.sh                    # trigger the Release workflow for origin/master's version

scripts/git-release.sh --check-only   # run every check, change nothing
scripts/git-release.sh                # all of the above in one step: check, bump, commit, tag, push, release
```

The Release workflow (`.github/workflows/release.yaml`) runs CI and the security checks,
builds all five targets in parallel, uploads them to a draft release with
`SHA256SUMS.txt`, publishes it, then runs the action end to end against the new release
with `fail-on-warn`.
