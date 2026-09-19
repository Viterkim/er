# Lazy

I don't recommend this for normal app code, or really anything. Read the [README](../../README.md) and the [examples](examples.md) first. This is for small scripts or prototyping, i really believe adding small types with the macros is easy, so this should not be needed.

For tests, use the [`test` feature](examples.md#tests), not this:

```toml
[dev-dependencies]
er = { version = "0.2", features = ["test"] }
```

Then your tests can return `ErTest` and use `.er(())?`.

## Using it

```toml
[dependencies]
er = { version = "0.2", features = ["lazy"] }
```

## Examples

```rust
use er::*;
use std::fs;

// Just where it happened
fn read_config() -> ErLazy<String> {
    fs::read_to_string("bingo/file.toml").er(())
}

// String context
fn load() -> ErLazy<String> {
    read_config().er(|| "loading config")
}

// Already added context, it will NOT add anything more, be careful
fn run() -> ErLazy {
    let config = load()?;
    println!("{config}");
    Ok(())
}
```

If `bingo/file.toml` isn't there, `read_config().unwrap_err().er_report()` prints:

```text
failed @ src/main.rs:6:43
`- No such file or directory (os error 2)
```

And `run().unwrap_err().er_report()` prints:

```text
loading config @ src/main.rs:11:19
`- failed @ src/main.rs:6:43
   `- No such file or directory (os error 2)
```
