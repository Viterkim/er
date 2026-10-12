#!/usr/bin/env bash
set -euo pipefail
cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.."

cargo fmt --manifest-path integrations/Cargo.toml --all --check
cargo test --manifest-path integrations/Cargo.toml --workspace --exclude compile-tests
cargo test --manifest-path integrations/Cargo.toml -p compile-tests --test compile accept -- --exact

pinned_toolchain=$(sed -n 's/^channel = "\(.*\)"/\1/p' rust-toolchain.toml)
cargo +"$pinned_toolchain" test --manifest-path integrations/Cargo.toml -p compile-tests

cargo +1.89.0 test --manifest-path integrations/Cargo.toml -p compile-tests --test compile accept -- --exact
cargo +1.89.0 test --manifest-path integrations/Cargo.toml -p boundary
cargo check --manifest-path integrations/Cargo.toml -p unsync --target thumbv6m-none-eabi

cargo fmt --manifest-path integrations/no-src-locations/Cargo.toml --all --check
cargo test --manifest-path integrations/no-src-locations/Cargo.toml
