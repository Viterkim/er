#!/usr/bin/env bash
set -euo pipefail
cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.."

cargo fmt --manifest-path integrations/Cargo.toml --all --check
cargo test --manifest-path integrations/Cargo.toml --workspace

cargo fmt --manifest-path integrations/no-src-locations/Cargo.toml --all --check
cargo test --manifest-path integrations/no-src-locations/Cargo.toml

