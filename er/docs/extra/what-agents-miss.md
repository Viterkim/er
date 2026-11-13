# What agents miss

Most Rust code agents have seen uses anyhow or thiserror. Reach for those patterns here and you can write code that compiles while quietly throwing away the bit we use Er for.

## Never rebuild a tree with map_err

```rust
// Bad, now all we have is the code.
read_device().map_err(|error| er_new!(AnalyzeErr::new(error.top.code)))

// Good, our error on top and the original failure still there.
read_device().er_with(|error| error.code)
```

To make a failure from scratch and keep it for later use `er_new!`. When a call already gave you a failure, add to it. At the public boundary, `.er_split()` gives your caller `(MyErr, String)`.

## Match first, print in the Err arm

```rust
match load_config(path) {
    Ok(config) => use_config(config),
    Err(error) => eprintln!("{}", error.er_report()),
}
```

Putting `.er_report()` before the match turns the failure into a String. The branch that ought to match on `error.top` now has to guess from words. A retry should work even after someone rewrites the message or moves an inner error. Put the case on your error at the call site where you still know what it means.

## Stop telling Rust which error it is

```rust
// Bad
let config = read_config(path).er::<LoadErr>(())?;
let result: ErResult<String, LoadErr> = read_config(path).er(());

// Good
let config = read_config(path).er(())?;
```

Your function already returns `ErResult<_, LoadErr>`, so Rust knows which error it is, and the `LoadErr` is just noise you read past every time. The turbofish is for when nothing says which error it is, like a sub error inside `er_all!` or the middle of a chain, `.er::<OpenErr>(path).er(())?`.

## You don't need a closure for a string

```rust
// Bad
fs::read_to_string(path).er(|_| (path, "load config"))

// Good
fs::read_to_string(path).er((path, "load config"))
```

The path is already in your hand. Pass it in and Er does the conversion on failure. A call like `user.name()` goes inside `.er(|_| (path, user.name()))` so a successful read skips the lookup too.

## Don't wrap the error back up in Err

```rust
// Bad
Err(error) => return Err(error).er(|| LoadErr::open(&path)),

// Good
Err(error) => er_bail!(LoadErr::open(&path), error),
```

Give `er_bail!` your new error and the original failure. Use `er_new!(LoadErr::open(&path), error)` when you want the tree back to keep working with it.

## Don't make ErShared up front

```rust
// Bad
let path = ErShared::new(workspace_path(&paths));
fs::read_to_string(&*path).er(|_| path.clone())?;

// Good
let path = workspace_path(&paths);
fs::read_to_string(&path).er(&path)?;
```

The bad one allocates every time, even though it almost never fails. `.er(&path)` only makes the `ErShared` when it does.

## Clone what you carry up in er_with

```rust
// Bad, "lifetime may not live long enough"
read_file(path).er_with(|err| (&err.path, machine))

// Good
read_file(path).er_with(|err| (err.path.clone(), machine))
```

A closure can't hand back a borrow of its own argument, so clone it. On an `ErShared` field the clone just bumps a counter.
