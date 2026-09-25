# Features

The crate is `no_std` but requires `alloc`. `stack_traces` needs `std`.

## default features

`macros, src_locations`

## non-default features

`small_path_src, serde, lazy, stack_traces`

## non-default dev/testing feature

`test`

## macros

Convenience macros like `#[derive(Er)]`, `#[derive(ErFormat)]`, `er_all!` and the `er_bail!` family.

## src_locations

Enables capturing the file path, line number and column number at compile time.

## small_path_src, off by default

Usually you'll just get `src/lib.rs` or `engine/src/lib.rs` in a workspace(so you WONT need this), but on some setups if you use paths for dependencies, like a local crate on your pc, then you'll get the full path `/the/magic/src/folder/that/was/supposed/to/be/secret/src/a.rs`.

So this turns it into `secret/src/a.rs`. It just looks for the last `src` and gives you the path one step back from that.

If you don't want to restructure stuff you can enable it, but it only affects it when printing (it's still in your binary).

## serde, off by default

This only adds Serialize/Deserialize snapshots(converted string reports).

Er does NOT turn it into JSON for you. You pick a format crate in your own app `serde_json`, `toml` etc.

```toml
[dependencies]
er = { version = "0.3", features = ["serde"] }
serde_json = "1"
```

```rust
// Just unwrapping for the example
let snapshot = read_port("nope").unwrap_err().er_snapshot();

let json = serde_json::to_string_pretty(&snapshot).unwrap();
fs::write("/tmp/error.json", &json).unwrap();

let snapshot: ErSnapshot = serde_json::from_str(&json).unwrap();
println!("{}", snapshot.er_report());
```

If one side compiled `src_locations` out, the JSON just has no `src_location`. A locations build loads that as `None` (no `@ file:line`). Extra keys the other way get ignored.

## lazy, off by default

For small scripts or prototyping, i don't think you should use this [Examples](lazy.md).

## stack_traces, off by default

Needs `std`. [Capturing and printing traces](examples.md#stack-traces).

## test, for dev dependencies, off by default

`ErTest` and `.er(())?` for tests.

## Examples

```toml
[dependencies]
er = { version = "0.3", features = ["small_path_src"] }

[dev-dependencies]
er = { version = "0.3", features = ["test"] }
```

```toml
[dependencies]
er = { version = "0.3", default-features = false, features = ["src_locations"] }
```
