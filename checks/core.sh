#!/usr/bin/env bash
set -euo pipefail
cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.."

cargo fmt --all --check

cargo +1.89.0 check --workspace --all-features --all-targets
cargo +1.89.0 check -p er --no-default-features --all-targets

# Pass +1.89.0 (or nothing for current)
test_core() {
    cargo "$@" test --workspace --all-features
    cargo "$@" test -p er --no-default-features --test api --test snapshot
    cargo "$@" test -p er --no-default-features --features src_locations,test --test api
    cargo "$@" test -p er --no-default-features --features macros,test
    cargo "$@" test -p er --no-default-features --features macros,src_locations --test format
    cargo "$@" test -p er --no-default-features --features serde --test snapshot
    cargo "$@" test -p er --no-default-features --features serde,src_locations --test snapshot
}

test_core +1.89.0
test_core

# Lints
cargo clippy --workspace --all-targets -- -D warnings
cargo clippy -p er --all-targets --no-default-features -- -D warnings
cargo clippy -p er --all-targets --no-default-features --features macros -- -D warnings
cargo clippy -p er --all-targets --no-default-features --features serde -- -D warnings

# Docs
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
RUSTDOCFLAGS="-D warnings" cargo doc -p er --no-default-features --no-deps
