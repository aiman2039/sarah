#!/bin/sh
# Shared local and CI quality checks. Clippy is the Rust linter; pedantic
# lints come from [lints.clippy] in Cargo.toml.
set -eu
cd "$(dirname "$0")/.."
need_plugin() {
    if ! cargo "$1" --version >/dev/null 2>&1; then
        echo "missing cargo-$1; install with: cargo install cargo-$1 --locked" >&2
        exit 1
    fi
}
case "${1:-all}" in
    fmt) cargo fmt --all --check ;;
    clippy) cargo clippy --all-targets --all-features --locked -- -D warnings ;;
    test) cargo test --all-features --locked ;;
    # sarah on its own code: any warning over sarah.toml / the default limits fails.
    sarah) cargo run --quiet --locked -- . --fail-on-warn --top 5 ;;
    audit)
        need_plugin audit
        cargo audit
        ;;
    deny)
        need_plugin deny
        cargo deny check advisories bans licenses sources
        ;;
    all)
        for check in fmt clippy test sarah audit deny; do
            sh "$0" "$check"
        done
        ;;
    *) echo "Usage: $0 [all|fmt|clippy|test|sarah|audit|deny]" >&2; exit 2 ;;
esac
