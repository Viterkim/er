# Examples

`use er::*;` at the top, then give each function that deals with errors its own `FuncNameEr` type with `#[derive(Er)]` and use `.er(...)` on results, errors and options.

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
pub struct ReadFileEr { // or `ReadFileEr(pub PathBuf)`
    pub path: PathBuf,
}
pub fn read_file(path: &Path) -> Er<String, ReadFileEr> {
    fs::read_to_string(path).er(|| ReadFileEr::new(path))
}
```

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

## Don't destroy the tree (lose sub errors)

You should use `.er_with(|e|)` from the example below this (`Look at the previous error`).

But here's an example of the manual way, and how easy it is to destroy the tree.

```rust
#[derive(Er)]
pub struct AnalyzeEr;
pub fn analyze() -> Er<(), AnalyzeEr> {
    if let Err(previous_error_tree) = read_port("nope") {
        // Bad: new error, previous tree is gone
        // BAD: return Err(AnalyzeEr::new().er());

        // Good: manually add context to the existing tree
        // BUT use .er_with(|e|) instead, it does it for you.
        return Err(previous_error_tree.er(AnalyzeEr::new));
    }
    Ok(())
}
```

Same thing with `map_err`:

```rust
// BAD: we ignore the previous error, tree disappears
read_port(input).map_err(|_previous_error_tree| AnalyzeEr::new().er())

// Good
read_port(input).er(AnalyzeEr::new)
#[derive(Er)]
pub struct AnalyzeEr2(pub u32);
read_port(input).er_with(|e |AnalyzeEr::new(e.thing_you_needed))
```

## Look at the previous error

If you need some value on the previous error (and don't want to .find()).

You can just use `.er(||)` if you don't need the value of the error below, in this error itself.

`.er_with(|e|)` keeps the tree, so is easier than using `.map_err(||)` and accidentally destroying trhe tree.

If it's already an Er tree (`Er<T, DeviceEr>` / `ErTree<DeviceEr>`), the `|t|` is the tree. Your typed error is `t.top`.

```rust
#[derive(Er)]
pub struct DeviceEr {
    pub code: u8,
}
#[derive(Er)]
pub struct AnalyzeEr {
    pub code: u8,
}
pub fn analyze() -> Er<(), AnalyzeEr> {
    read_device().er_with(|t| AnalyzeEr::new(t.top.code))?;
    Ok(())
}
```

If already a tree (same):

```rust
return Err(tree.er_with(|t| AnalyzeEr::new(t.top.code)));
```

If its a plain error (not a tree yet), then the `|e|` is the error:

```rust
return Err(device.er_with(|e| AnalyzeEr::new(e.code)));
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

For a struct inside your error, `ErFormat` gives the same Display/Debug and constructors, but no `Error`.

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

let connection = Connection::new("ComputerKatten", 85);
let error = ConnectEr::new(connection).er();

println!("{}", error.er_top());
```
```text
ConnectEr { connection: Connection { host: "ComputerKatten", port: 85 } }
```

## Collect/aggregate

Collects multi failures (different types are fine).

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

Or if the collections is there already.

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

Or you want to keep the report as text too.
```rust
#[derive(Er)]
pub struct ApiError {
    pub report: String,
    pub err_msg: String,
}

pub fn public_read_port(input: &str) -> Result<u16, ApiError> {
    // Remember to avoid using `.map_err()` in Er for most cases.
    // We're turning the tree into text (it gets dropped), so its what we actually want here.
    read_port(input).map_err(|error| ApiError {
        report: error.er_report().to_string(),
        err_msg: "invalid port".to_string(),
    })
}

if let Err(error) = public_read_port("fakenumber") {
    // !WARNING! Don't just print "{error:?}" you'll get `\n` instead of actual newlines    
    println!("{}", error.report);
    println!("{}", error.err_msg);
}
```

## Non errors (values)

NEVER do this on a tree, you nuke it! !This makes a new tree!

For values that don't implement `Error` like `Err(85)`.

```rust
#[derive(Er)]
pub struct DeviceEr {
    pub status: u8,
}
pub fn check_device(result: Result<(), u8>) -> Er<(), DeviceEr> {
    // Remember, the error is a value and gets passed into the first argument
    // Same as `|v| DeviceEr::new(v)`
    result.er_from_val(DeviceEr::new)
}
```

## Tests

Enable `test` on your dev deps:

```toml
[dev-dependencies]
er = { version = "0.1", features = ["test"] }
```

Make the test return `TestEr` and use `.t_er()?`.

```rust,ignore
use er::*;

#[derive(Er)]
pub struct ReadPortEr;
pub fn read_port(input: &str) -> Er<u16, ReadPortEr> {
    input.parse().er(ReadPortEr::new)
}

#[test]
pub fn the_best_test() -> TestEr {
    read_port("nope").t_er()?;
    Ok(())
}
```
```text
Error: TestEr @ tests/the_best_test.rs:12:23
`- ReadPortEr @ tests/the_best_test.rs:7:19
   `- invalid digit found in string @ tests/the_best_test.rs:7:19
test the_best_test ... FAILED
```

## Other traits

If you need to implement another crate's trait on the whole tree, use Wrap.

```rust
#[derive(Er)]
#[er(wrap(name = HandlerError))] // Defaults to HandlerErWrap without name
pub struct HandlerEr;
pub fn handler(input: &str) -> Result<u16, HandlerError> {
    let port = input.parse().er(HandlerEr::new)?;
    Ok(port)
}
```

`?` puts the tree in `HandlerError`. Add context as usual:

```rust
handler(input).er(RequestEr::new)
```

[Wrap options and the trait impl](macros.md#wrap).

## Snapshots

Saves msgs and the tree structure for christmas or other special occasions.

Remember they aren't real errors but strings.

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

Can still print `.er_top()` or `.er_report()`. Each entry has its parent and depth.

Enable `serde` on Er, then add `serde_json` (or toml, or whatever) in your own crate if you want to save it. Er doesn't have `to_json()`.

```toml
[dependencies]
er = { version = "0.1", features = ["serde"] }
serde_json = "1"
```

```rust
let snapshot = read_port("fakenumber").unwrap_err().er_snapshot();
let json = serde_json::to_string_pretty(&snapshot).unwrap();
std::fs::write("/tmp/error.json", &json).unwrap();
```

## If you want Error on report (opaque)

You can make your own public type and attach a string report, or a snapshot, check the `Public error` example above.

But if you want to force the real report, into an Error for like anyhow, then you can.

```rust
pub fn run() -> anyhow::Result<()> {
    // Or `.er_top()`
    read_port("nope").er_report().opaque_err()?;
    Ok(())
}
```

BONUS: Nothing is deleted `ErAsError` keeps the presentation in its public `.0` field.

BUT `.opaque_err()` stops searches (source() is empty), and going the other way, Anyhow's boxed conversion can also be a sneaky bitch and hide types from er_find. [The anyhow example](../../integrations/anyhow/src/lib.rs) shows both.

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

`#[er(no_constructors)]` on the struct/enum skips `new` and all variant constructors, for either derive.

[More about the macros](macros.md), including the generated constructors.
