#!/usr/bin/env bash
set -euo pipefail

cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.."

if [[ $# != 1 || ! $1 =~ ^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$ ]]; then
    printf 'Usage: %s 1.2.3\n' "$0" >&2
    exit 1
fi

version=$1
doc_version=${version%.*}
anchor_version=${version//./}
sed -i -E "s/^version = \"[^\"]+\"$/version = \"$version\"/" Cargo.toml
sed -i -E "s/^(er-macros = \{ version = \"=)[^\"]+/\1$version/" er/Cargo.toml er-unsync/Cargo.toml

shopt -s globstar nullglob
for doc in README.md er-unsync/README.md er-macros/README.md er/docs/**/*.md; do
    [[ $doc == */changelog.md ]] && continue
    sed -i -E \
        -e "s/^((er|er-macros|er-unsync) = \")[^\"]+/\1$doc_version/" \
        -e "s/^((er|er-macros|er-unsync) = \{ version = \")[^\"]+/\1$doc_version/" \
        -e "s/^(er = \{ package = \"er-unsync\", version = \")[^\"]+/\1$doc_version/" \
        -e "s/^(#{1,6} Er \()[0-9]+\.[0-9]+(\.[0-9]+)?\)/\1$version)/" \
        -e "s/(#er-)[0-9]+/\1$anchor_version/g" \
        -e "s@(https://docs\.rs/(crate/)?(er|er-macros|er-unsync)/)(latest|[0-9]+\.[0-9]+(\.[0-9]+)?)/@\1$version/@g" \
        "$doc"
done

# Only our versions (no upgrading other dependencies)
for manifest in Cargo.toml integrations/Cargo.toml integrations/no-src-locations/Cargo.toml er-macros/bench/Cargo.toml; do
    cargo metadata --offline --format-version 1 --manifest-path "$manifest" > /dev/null
done

printf 'Er + er-unsync + er-macros are now %s\n' "$version"
