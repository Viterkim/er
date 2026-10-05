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
release_url="https://github.com/Viterkim/er/blob/$(git rev-parse HEAD)"

# the `er-unsync` crate shares the source here, its published package gets its own copy.
cp -R "$release_dir/er/src" "$release_dir/er-unsync/src"
sed -i 's@path = "../er/src/lib.rs"@path = "src/lib.rs"@' "$release_dir/er-unsync/Cargo.toml"

# Only the published version gets full links
shopt -s globstar nullglob
link_pattern='\]\(([^)]+)\)'
repo_url='https://github.com/Viterkim/er/blob/main/'

for doc in "$release_dir/README.md" "$release_dir/er-macros/README.md" "$release_dir/er-unsync/README.md" "$release_dir"/er/docs/**/*.md; do
    doc_copy=$(mktemp "$doc.XXXXXX")

    while IFS= read -r line || [[ -n "$line" ]]; do
        output=
        while [[ $line =~ $link_pattern ]]; do
            matched_link=${BASH_REMATCH[0]}
            target=${BASH_REMATCH[1]}
            output+=${line%%"$matched_link"*}
            line=${line#*"$matched_link"}

            case "$target" in
                "$repo_url"*) target="$release_url/${target#"$repo_url"}" ;;
                \#* | *:* | //*) ;;
                *)
                    target_path=${target%%[?#]*}
                    suffix=${target#"$target_path"}
                    source_path="$(dirname -- "$doc")/$target_path"

                    if [[ ! -e "$source_path" ]]; then
                        printf '%s: missing link %s\n' "${doc#"$release_dir/"}" "$target" >&2
                        exit 1
                    fi

                    target_path=$(realpath --relative-to="$release_dir" -- "$source_path")
                    target="$release_url/$target_path$suffix"
                    ;;
            esac

            output+="]($target)"
        done
        printf '%s\n' "$output$line"
    done < "$doc" > "$doc_copy"

    cat "$doc_copy" > "$doc"
    rm -- "$doc_copy"
done

cd -- "$release_dir"
cargo publish --workspace --dry-run

printf '\nDry run bingo! You are in %s\n' "$release_dir"
printf 'Bash:'
printf 'cargo publish -p er-macros && cargo publish -p er && cargo publish -p er-unsync\n'
printf 'Nushell:'
printf 'cargo publish -p er-macros; cargo publish -p er; cargo publish -p er-unsync\n'
printf 'exit this shell when you are done.\n\n'
exec "${SHELL:-bash}" -i
