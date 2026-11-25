# Features

The crate is `no_std` but requires `alloc`.

## default features

`macros, src_locations`

## non-default features

`std, small_path_src, stack_traces`

## std, off by default

Lets an `ErShared<PathBuf>` field take a `&Path`, like a plain `PathBuf` field does. Without it you pass a `&PathBuf`.

## macros

Convenience macros like `#[derive(Er)]`, `#[derive(ErFormat)]`, `er_new!`, `er_all!`, `er_try!`, `er_add!`, `er_bail!` and `er_assert_err!`.

## src_locations

Enables capturing the file path, line number and column number at compile time.

## small_path_src, off by default

Usually you'll just get `src/lib.rs` or `engine/src/lib.rs` in a workspace(so you WONT need this), but on some setups if you use paths for dependencies, like a local crate on your pc, then you'll get the full path `/the/magic/src/folder/that/was/supposed/to/be/secret/src/a.rs`.

So this turns it into `secret/src/a.rs`. It just looks for the last `src` and gives you the path one step back from that.

If you don't want to restructure stuff you can enable it, but it only affects it when printing (it's still in your binary).

## stack_traces, off by default

Turns on `std`. [Capturing and printing traces](examples.md#stack-traces).

## Examples

```toml
[dependencies]
er = { version = "0.9", features = ["small_path_src"] }
```

```toml
[dependencies]
er = { version = "0.9", default-features = false, features = ["src_locations"] }
```

## Unsync / Single threaded platform

If you've got a platform that uses Rc/RefCell you can use the `er-unsync` version of the crate. Keep the name `er` so the derives can find it.

```toml
er = { package = "er-unsync", version = "0.9" }
```
