# Lazy

I don't recommend this for normal code. Read the [README](../../../README.md) and the [examples](../examples.md) first.

This is for small scripts or prototyping, but even then it's easy to add types, so this is not needed.

For tests, use [`ErTest`](../examples.md#tests)

## Using it

```toml
[dependencies]
er = { version = "0.6", features = ["lazy"] }
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
    read_config().er(|_| "loading config")
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
