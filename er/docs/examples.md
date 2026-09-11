# Examples

`use er::*;` at the top, then give each function that deals with errors its own `FuncNameEr` ish type with `#[derive(Er)]` and use `.er(...)` on results, errors and options.

Don't throw away the tree in `map_err`. Use `.er(...)` for context.

The TLDR is `#[derive(Er)]YourEr + return Er<(), YourEr> + .er(||)`

## Empty struct

If the name and location are enough:

```rust
#[derive(Er)]
pub struct ReadPortEr;
pub fn read_port(input: &str) -> Er<u16, ReadPortEr> {
    input.parse().er(ReadPortEr::new)
}
```

## Data struct

Keep the stuff the caller cares about:

```rust
#[derive(Er)]
pub struct ReadFileEr {
    pub path: PathBuf,
}
pub fn read_file(path: &Path) -> Er<String, ReadFileEr> {
    fs::read_to_string(path).er(|| ReadFileEr::new(path))
}
```

Rc/Cell can stay in the top error. Putting that error below another needs `Send + Sync + 'static`.

## Enum

Variants get their own constructors:

```rust
#[derive(Er)]
pub enum ModeEr {
    Missing,
    Unknown { input: String },
}
pub fn read_mode(input: Option<&str>) -> Er<&str, ModeEr> {
    // Even on options (like .ok_or_else())
    let mode = input.er(ModeEr::missing)?;

    if mode != "haandbold" {
        return Err(ModeEr::unknown(mode).er());
    }

    Ok(mode)
}
```

## Print report

```rust
if let Err(error) = read_port("fakenumber") {
    eprintln!("{}", error.er_report());
}
```

Works with `.expect()` too:

```rust
let port = read_port("85").er_report().expect("usable port");
```

`main` can also just return `Result<(), ErReport<AppEr>>`.

## Print top error

Just the top error:

```rust
if let Err(error) = read_file(Path::new("missing85")) {
    eprintln!("{}", error.er_top());
    println!("Missing: {}", error.top.path.display());
}
```

```text
ReadFileEr { path: "missing85" }
Missing: missing85
```

`error.top` is still your type, just read its fields. `.er_top()` only chooses what gets printed.

## Type inside

For a struct inside your error, `ErFormat` gives normal data the same Display and Debug formatting (doesn't implement `Error` and doesn't make any constructors).

```rust
#[derive(ErFormat)]
pub struct Connection {
    pub host: String,
    pub port: u16,
}

#[derive(Er)]
pub struct ConnectEr {
    pub connection: Connection,
}

let connection = Connection {
    host: "ComputerKatten".into(),
    port: 85,
};
let error = ConnectEr::new(connection).er();

println!("{}", error.er_top());
```
```text
ConnectEr { connection: Connection { host: "ComputerKatten", port: 85 } }
```

## Collect/aggregate

Collects both failures, different types are fine.

```rust
#[derive(Er)]
pub struct ConfigEr {
    pub port: String,
    pub enabled: String,
}
pub fn check_config(port: &str, enabled: &str) -> Er<(), ConfigEr> {
    er_all!(
        || ConfigEr::new(port, enabled),
        [port.parse::<u16>(), enabled.parse::<bool>()],
    )
}
```

Already have a Vec? Same call:

```rust
#[derive(Er)]
pub struct FilesEr;
pub fn read_files(paths: &[PathBuf]) -> Er<(), FilesEr> {
    let mut results = Vec::new();

    for path in paths {
        results.push(read_file(path));
    }

    er_all!(FilesEr::new, results)
}
```

## Find original error

```rust
if let Err(error) = read_file(Path::new("missing85")) {
    if let Some(source) = error.er_find::<io::Error>() {
        println!("{:?}", source.kind());
    }
}
```

Also checks `source()`. For all matches:

```rust
for failed in error.er_find_all::<ReadFileEr>() {
    println!("{}", failed.path.display());
}
```

Native `source()` chains stop after 256 hops per stored error, so a loop can't hang the walk. Searches won't see past that limit. Reports show `[source limit reached]`, and live/saved entries set `source_truncated`. Ordinary Er tree depth isn't limited.

## Public error

The consumer doesn't need Er. `#[derive(Er)]` also makes a normal Rust error. `.er()` is the part that makes a tree.

```rust
#[derive(Er)]
pub enum ApiError {
    Missing,
    #[er(format = "Not a port: {input}")]
    InvalidPort { input: String },
}

// Consumer doesn't need to have er, it's just a normal error for them.
let error = ApiError::invalid_port("fakenumber");
match error {
    ApiError::Missing => println!("Where port?"),
    ApiError::InvalidPort { input } => println!("Try 85, not {input}"),
}
```

## Non error fails

For stuff like `Err(85)`, where 85 isn't an Error:

```rust
#[derive(Er)]
pub struct DeviceEr {
    pub status: u8,
}
pub fn check_device(result: Result<(), u8>) -> Er<(), DeviceEr> {
    result.er_from(DeviceEr::new)
}
```

Starts a new tree. Usually you want `.er(...)` to keep the old one.

## Tests

Enable `test` on your dev deps:

```toml
[dev-dependencies]
er = { version = "0.1", features = ["test"] }
```

Then use `TestEr` and `.t_er()?`:

```rust,ignore
#[test]
pub fn port() -> TestEr {
    let port = read_port("85").t_er()?;

    assert_eq!(port, 85);
    Ok(())
}
```

## Wrap / External traits

!Errors from other crates don't need Wrap! Just use `.er(...)`.

But for times where the orphan rule hits you, and you need to implement another crate's trait (hello Axum's `IntoResponse`):

```rust
#[derive(Er)]
#[er(wrap(name = HandlerError))] // Defaults to HandlerErWrap without name
pub struct HandlerEr;
pub fn handler(input: &str) -> Result<u16, HandlerError> {
    let port = input.parse().er(HandlerEr::new)?;
    Ok(port)
}
```

`.er_find()`, `.er_top()` and `.er_report()` still work. [The Axum example](../../integrations/axum/src/lib.rs) shows the trait impl.

If you need `Error` or `Display/Debug` add `output = report` or `output = top` which implements that style and implements `Error`: `#[er(wrap(name = HandlerError, output = report))]`.

Use it when another library needs those traits, or when you need a local type for your own trait impl. Normal .er(...) calls don't need Wrap.

And remember, Er wants you to pick report/top. If you call .er_report() or .er_top() on a Wrap, you still get Error + Display/Debug. 'output = report/top' is only for places that need it.

## Wrap and back

From Wrap back to Er:

```rust
#[derive(Er)]
pub struct RequestEr;
pub fn request(input: &str) -> Er<u16, RequestEr> {
    handler(input).er_tree().er(RequestEr::new)
}
```

Same tree with RequestEr added on top, use for owned reports and tops too, their layout is gone.

If you call `.er()` on a result with a Wrap with output makes the wrapper boxed which hides its inner errors from find, so use `.er_tree().er(...)`. 

Anyhow doesn't need wrap but its boxed conversion can be a sneaky bitch and hide types from `er_find`, so you might have to do [something like the Anyhow example here](../../integrations/anyhow/src/lib.rs).

## Snapshots

Saved messages and tree structure for christmas or other special occasions.

Keep the messages and structure, let the real errors go (i know it's hard, i believe in you):

```rust
if let Err(error) = read_port("fakenumber") {
    let snapshot = error.er_snapshot();
    drop(error);

    println!("{}", snapshot.er_report());

    for entry in snapshot.er_entries() {
        println!("{}", entry.message);
    }
}
```

Clone it, store a Vec of em', print `.er_top()` or `.er_report()`. Each entry has its parent and depth, not just a printed line. (Disclaimer: The original error types are gone though).

## Macros

Custom text does both Display and Debug. `exact` takes the field type directly instead of `impl Into<T>`:

```rust
pub type Port = u16;

#[derive(Er)]
#[er(format = "Couldn't connect to {host} on port {port}")]
pub struct ConnectEr {
    pub host: String,
    #[er(exact)]
    pub port: Port,
}

let error = ConnectEr::new("ComputerKatten", 85);
println!("{error}");
```

`{field}` uses Display, `{field:?}` uses Debug. Tuple fields use `{0}` and `{1:?}`. Put `format` on a struct or enum variant.

`#[er(skip)]` leaves a field out. `#[er(censor)]` prints `*CENSORED*` (data still there). 

[More about the macros](https://github.com/Viterkim/er/blob/main/er/docs/macros.md), including the generated constructors.
