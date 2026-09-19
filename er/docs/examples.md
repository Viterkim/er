# Examples

`use er::*;` at the top, then give each function that deals with errors its own `FuncNameErr` type with `#[derive(Er)]` and use `.er()` on results, errors and options.

## Empty struct

If the name and location are enough:

```rust
#[derive(Er)]
pub struct ReadPortErr;
pub fn read_port(input: &str) -> Er<u16, ReadPortErr> {
    input.parse().er(())
}
```

## Data struct

Keep the stuff the caller cares about:

```rust
#[derive(Er)]
pub struct ReadFileErr { // or `ReadFileErr(pub PathBuf)`
    pub path: PathBuf,
}
pub fn read_file(path: &Path) -> Er<String, ReadFileErr> {
    fs::read_to_string(path).er(|| path)
}
```

## Enum

Variants get their own constructors:

```rust
#[derive(Er)]
pub enum ModeErr {
    Missing,
    Unknown { input: String },
}
pub fn read_mode(input: Option<&str>) -> Er<&str, ModeErr> {
    // Even on options (like .ok_or_else())
    let mode = input.er(ModeErr::missing)?;

    if mode != "haandbold" {
        return Err(ModeErr::unknown(mode).er());
    }

    Ok(mode)
}
```

## Don't destroy the tree (lose sub errors)

`.er()` keeps the old tree below your new error. The easy way to lose it is to make a fresh error in `map_err`.

```rust
#[derive(Er)]
pub struct AnalyzeErr;
pub fn analyze() -> Er<(), AnalyzeErr> {
    if let Err(previous_error_tree) = read_port("nope") {
        // BAD: return Err(AnalyzeErr::new().er()); // old tree gone
        // Good: add to the tree we already have
        return Err(previous_error_tree.er(()));
    }
    Ok(())
}
```

Same thing with `map_err`, using the types from the next example:

```rust
// BAD: we copied the code, but threw away the old error and its tree
read_device().map_err(|t| AnalyzeErr::new(t.top.code).er())

// Good: copies the code AND keeps the old error and its tree
read_device().er_with(|t| t.top.code)
```

## Look at the previous error

If you need something from the old error, `.er_with()` lets you look at it and still keeps it in the tree. If it's already an Er tree, `t.top` is the error you made:

```rust
#[derive(Er)]
pub struct DeviceErr {
    pub code: u8,
}
#[derive(Er)]
pub struct AnalyzeErr {
    pub code: u8,
}
pub fn analyze() -> Er<(), AnalyzeErr> {
    read_device().er_with(|t| t.top.code)?;
    Ok(())
}
```

If its a plain error (not a tree yet), then the `|e|` is the error:

```rust
return Err(device.er_with(|e| e.code));
```

If you need to own something from the old error, use `.clone()`, `.to_string()`, whatever Rust needs there.

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

`main` can also just return `Result<(), ErReport<AppErr>>`.

## Print top error

Just the top error:

```rust
if let Err(error) = read_file(Path::new("missing85")) {
    eprintln!("{}", error.er_top());
    println!("Missing: {}", error.top.path.display());
}
```

```text
ReadFileErr { path: "missing85" }
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
pub struct ConnectErr {
    pub connection: Connection,
}

let connection = Connection::new("ComputerKatten", 85);
let error = ConnectErr::new(connection).er();

println!("{}", error.er_top());
```
```text
ConnectErr { connection: Connection { host: "ComputerKatten", port: 85 } }
```

## Collect/aggregate

Collects multi failures (different types are fine).

```rust
#[derive(Er)]
pub struct ConfigErr {
    pub port: String,
    pub enabled: String,
}
pub fn check_config(port: &str, enabled: &str) -> Er<(), ConfigErr> {
    er_all!(
        || (port, enabled),
        [port.parse::<u16>(), enabled.parse::<bool>()],
    )
}
```

Or if the collections is there already.

```rust
#[derive(Er)]
pub struct FilesErr;
pub fn read_files(paths: &[PathBuf]) -> Er<(), FilesErr> {
    let mut results = Vec::new();

    for path in paths {
        results.push(read_file(path));
    }

    er_all!((), results)
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
for failed in error.er_find_all::<ReadFileErr>() {
    println!("{}", failed.path.display());
}
```

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

## A tricky example

Foreign errors, own errors, the original error, string context, typed context, using / consuming, public boundary

```rust
#[derive(Er)]
pub enum ListenErr {
    InvalidInput { input: String },
    SacredPort,
    BindFailed { address: SocketAddrV4, kind: io::ErrorKind, available_ports: Vec<u16> },
}

pub fn listen(input: &str) -> Er<TcpListener, ListenErr> {
    let (ip, port) = input.split_once(':').unwrap_or((input, ""));

    // First 2 errors, to us they're both just bad input
    let ip = ip.parse::<Ipv4Addr>().er(|| ListenErr::invalid_input(input))?;
    let port = port.parse::<u16>().er(|| ListenErr::invalid_input(input))?;

    // Third error, our own rule that port 85 is sacred
    if port == 85 {
        return Err(ListenErr::sacred_port().er());
    }

    // Fourth error, we might want to match on what happened
    let address = SocketAddrV4::new(ip, port);
    TcpListener::bind(address).er_with(|e| {
        let available_ports = find_available_ports(address);
        ListenErr::bind_failed(address, e.kind(), available_ports)
    })
}

// Someone using our public API doesn't need Er.
// I use 'Err' for internal errors, and 'Error' for public facing ones.
pub type ListenError = ListenErr;
pub fn public_error_example(input: &str) -> Result<TcpListener, ListenError> {
    listen(input).map_err(|error| error.top)
}

// Using it ourselves (still with Er)
fn main() {
    match listen("127.0.0.1:8080") {
        Ok(listener) => start_server(listener),
        Err(error) => {
            match &error.top {
                ListenErr::InvalidInput { input } => eprintln!("bad input: {input}"),
                ListenErr::SacredPort => eprintln!("port 85 is sacred"),
                ListenErr::BindFailed { address, kind: io::ErrorKind::AddrInUse, available_ports } => {
                    eprintln!("{address} is already in use");
                    show_available_ports(available_ports);
                },
                ListenErr::BindFailed { address, .. } => eprintln!("couldn't bind {address}"),
            }

            // Below .top we have to search for the original error.
            if let Some(source) = error.er_find::<io::Error>() {
                eprintln!("raw error code: {:?}", source.raw_os_error());
            }
        }
    }
}
```

[The full tricky comparison](tricky-error-comparison.md) does this with the other libraries as a comparison.

## Non errors (values)

NEVER do this on a tree, you nuke it! !This makes a new tree!

For values that don't implement `Error` like `Err(85)`.

```rust
#[derive(Er)]
pub struct DeviceErr {
    pub status: u8,
}
pub fn check_device(result: Result<(), u8>) -> Er<(), DeviceErr> {
    // Remember, the error is a value and gets passed into the first argument
    // Same as `|v| DeviceErr::new(v)`
    result.er_val(DeviceErr::new)
}
```

## Tests

Enable `test` on your dev deps:

```toml
[dev-dependencies]
er = { version = "0.2", features = ["test"] }
```

Make the test return `ErTest` and use `.er(())?`.

```rust,ignore
use er::*;

#[derive(Er)]
pub struct ReadPortErr;
pub fn read_port(input: &str) -> Er<u16, ReadPortErr> {
    input.parse().er(())
}

#[test]
pub fn the_best_test() -> ErTest {
    read_port("nope").er(())?;
    Ok(())
}
```
```text
Error: ErTest @ tests/the_best_test.rs:12:23
`- ReadPortErr @ tests/the_best_test.rs:7:19
   `- invalid digit found in string
test the_best_test ... FAILED
```

## Other traits

If you need to implement another crate's trait on the whole tree, use Wrap.

```rust
#[derive(Er)]
#[er(wrap(name = HandlerError))] // Defaults to HandlerErrWrap without name
pub struct HandlerErr;
pub fn handler(input: &str) -> Result<u16, HandlerError> {
    let port = input.parse().er(())?;
    Ok(port)
}
```

`?` puts the tree in `HandlerError`. Add context as usual:

```rust
#[derive(Er)]
pub struct RequestErr;
pub fn request(input: &str) -> Er<u16, RequestErr> {
    handler(input).er(())
}
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
er = { version = "0.2", features = ["serde"] }
serde_json = "1"
```

```rust
let snapshot = read_port("fakenumber").unwrap_err().er_snapshot();
let json = serde_json::to_string_pretty(&snapshot).unwrap();
std::fs::write("/tmp/error.json", &json).unwrap();
```

## Opaque (edge case)

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
pub struct ConnectErr {
    pub host: String,
    #[er(exact)]
    pub port: Port,
}

let error = ConnectErr::new("ComputerKatten", 85);
println!("{error}");
```

`{field}` uses Display, `{field:?}` uses Debug. Tuple fields use `{0}` and `{1:?}`. Put `format` on a struct or enum variant.

`#[er(skip)]` leaves a field out. `#[er(censor)]` prints `*CENSORED*` (data still there). 

`#[er(no_constructors)]` on the struct/enum skips `new` and all variant constructors, for either derive.

[More about the macros](macros.md), including the generated constructors.
