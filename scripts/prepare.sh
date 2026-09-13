#!/usr/bin/env bash
set -euo pipefail
cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.."

# https://github.com/rust-lang/cargo/issues/16865
# Tfw hardcoded full paths, i want relative paths in my docs, so this is the shitty fix for this repo structure

if [[ -n "$(git status --porcelain)" ]]; then
    printf 'Commit first man\n' >&2
    exit 1
fi

bash scripts/check.sh

# !Checks can update lockfiles! DONT export an older committed version by accident.
if [[ -n "$(git status --porcelain)" ]]; then
    printf 'Checks changed files. Review and commit them first man\n' >&2
    exit 1
fi

release_dir=$(mktemp -d "${TMPDIR:-/tmp}/er-release.XXXXXX")
git archive HEAD | tar -xf - -C "$release_dir"

# Only the published version gets full links
sed -i -E 's@\]\((\./)?er/@](https://github.com/Viterkim/er/blob/HEAD/er/@g' "$release_dir/README.md"
# examples.md also becomes the rustdoc front page
sed -i -E \
    -e 's@\]\((\./)?macros\.md([)#])@](https://github.com/Viterkim/er/blob/HEAD/er/docs/macros.md\2@g' \
    -e 's@\]\(\.\./\.\./integrations/@](https://github.com/Viterkim/er/blob/HEAD/integrations/@g' \
    "$release_dir/er/docs/examples.md"
cd -- "$release_dir"
cargo publish --workspace --dry-run

printf '\nDry run bingo! You are in %s\n' "$release_dir"
printf 'cargo publish -p er-macros && cargo publish -p er\n'
printf 'exit this shell when you are done.\n\n'
exec "${SHELL:-bash}" -i
