# Features

Er uses `alloc` and works without `std`. The default features are `macros` and `src_locations`.

- `macros` gives you the derives and macros like `er_new!`, `er_bail!` and `er_all!`.
- `src_locations` records the file, line and column for the top and each stored sub error. Native `source()` entries use whatever location their own message gives.
- `small_path_src` shortens the locations printed by `src_locations`. A path like `/home/me/projects/engine/src/lib.rs` prints as `engine/src/lib.rs`, with the original path still in the binary.
- `stack_traces` needs `std` and lets you save backtraces. [Example](examples.md#stack-traces)

## er-unsync

Use `er-unsync` to keep `Rc` in sub errors or share trees on targets without pointer atomics. Keep the name `er` on the left so the derives can find it.

```toml
er = { package = "er-unsync", version = "0.9" }
```
