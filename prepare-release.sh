#!/usr/bin/env bash
set -euo pipefail
cd -- "$(dirname -- "${BASH_SOURCE[0]}")"

# https://github.com/rust-lang/cargo/issues/16865
# Tfw hardcoded full paths, i want relative paths in my docs, so this is the shitty fix for this repo structure

if [[ -n "$(git status --porcelain)" ]]; then
    printf 'Commit first man\n' >&2
    exit 1
fi

release_dir=$(mktemp -d "${TMPDIR:-/tmp}/er-release.XXXXXX")
git archive HEAD | tar -xf - -C "$release_dir"

# Only the published version gets full links
sed -i -E 's@\]\((\./)?er/@](https://github.com/Viterkim/er/blob/HEAD/er/@g' "$release_dir/README.md"
printf 'Ready for bingo/publish, cd into: %s\n' "$release_dir"
