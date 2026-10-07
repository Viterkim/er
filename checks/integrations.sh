#!/usr/bin/env bash
set -euo pipefail
cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.."

cargo fmt --manifest-path integrations/Cargo.toml --all --check
cargo test --manifest-path integrations/Cargo.toml --workspace
cargo check --manifest-path integrations/Cargo.toml -p unsync --target thumbv6m-none-eabi

cargo fmt --manifest-path integrations/no-src-locations/Cargo.toml --all --check
cargo test --manifest-path integrations/no-src-locations/Cargo.toml

