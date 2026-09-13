#!/usr/bin/env bash
set -euo pipefail
cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.."

if [[ $# != 1 || ! $1 =~ ^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$ ]]; then
    printf 'Usage: %s 0.1.x\n' "$0" >&2
    exit 1
fi

version=$1
sed -i -E "s/^version = \"[^\"]+\"$/version = \"$version\"/" Cargo.toml
sed -i -E "s/^(er-macros = \{ version = \"=)[^\"]+/\1$version/" er/Cargo.toml

# Only our versions (no upgrading other dependencies)
for manifest in Cargo.toml integrations/Cargo.toml integrations/no-src-locations/Cargo.toml er-macros/bench/Cargo.toml; do
    cargo metadata --offline --format-version 1 --manifest-path "$manifest" > /dev/null
done

printf 'Er + er-macros are now %s\n' "$version"
