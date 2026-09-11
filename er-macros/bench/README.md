# Macro benchmark

Run from the repo root:

```sh
RUST_MIN_STACK=16777216 cargo test -p er-macros --release --lib expansion_cost -- --ignored --nocapture
cargo build --manifest-path er-macros/bench/Cargo.toml --features derive --timings
```

Expansion alone, then parsing + expansion. Middle of three batches, includes dropping the output.

The build uses 500 unit errors, 100 structs, 1,000 variants, deep recursion and a few generic/Wrap cases. Drop `--features derive` for runtime only.

Use a fresh `--target-dir` if you want a clean build timing.
