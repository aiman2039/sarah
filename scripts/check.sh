#!/bin/sh
# Shared local and CI quality checks. Clippy is the Rust linter; pedantic
# lints come from [lints.clippy] in Cargo.toml.
set -eu
cd "$(dirname "$0")/.."
case "${1:-all}" in
    fmt) cargo fmt --all --check ;;
    clippy) cargo clippy --all-targets --all-features --locked -- -D warnings ;;
    test) cargo test --all-features --locked ;;
    all)
        for check in fmt clippy test; do
            sh "$0" "$check"
        done
        ;;
    *) echo "Usage: $0 [all|fmt|clippy|test]" >&2; exit 2 ;;
esac
