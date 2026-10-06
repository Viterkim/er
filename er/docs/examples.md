# Examples / Patterns

All examples use `use er::*;`

`#[derive(Er)]` implements `Error`, `Display`, `Debug` and helpers.

## TLDR

Structs: Use `.er(())` for empty structs, `.er(|_| path)` for 1 field structs, and `.er(|_| (port, enabled))` for 2 or more fields.

Enums: Need a variant specified like `.er(|| ModeErr::variant_name(arg1))`.

Most functions that deal with errors should have their own error type.

```rust
// Put your error type directly above your function.
// You don't have to do '#[er(format)]' and you don't have to 'hide' what's going on. In many cases, errors are for internal use.
#[derive(Er)]
pub struct MyFuncErr;

pub fn my_func() -> ErResult<(), MyFuncErr> {
  something_else().er(())?;
  // more real code
}

// Context which is relevant in your logs / to your consumer
#[derive(Er)]
pub struct MyOtherErr {
  pub path: PathBuf, // What file didn't exist
  pub msg: &'static str, // Bonus: even if it was an Option<>, Some(v)/.into() wouldn't be needed
}

pub fn other_func(path: &str) -> ErResult<(), MyOtherErr> {
  // Usually pass values directly, no .as_ref()/.as_str()
  if path.is_empty() {
    er_bail!(|_| (path, "no file given"));
  }
  my_func().er(|_| (path, "some cool msg"))
}
```

Your errors should not be giant pyramids, they should be local to the things you are doing, related to what your consumer cares about and not which errors you got (the original errors are always stored automatically in the tree below).

Think about what your caller wants to 'match on', or what is relevant for the flow/logic of your code. Don't make state that can't exist and instead make variants with exactly why/what happened so the consumer can print it or react on it.

You can carry data up instead of runtime searching(`.er_find()`) by using `.er_with(|err|)` to inspect the previous error. You can also use `ErShared<DataType>` on your fields in your error types to avoid cloning the value.

## Empty struct

If the name and location are enough:

```rust
#[derive(Er)]
pub struct ReadPortErr;

pub fn read_port(input: &str) -> ErResult<u16, ReadPortErr> {
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

pub fn read_file(path: &Path) -> ErResult<String, ReadFileErr> {
    fs::read_to_string(path).er(|_| path)
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

pub fn read_mode(input: Option<&str>) -> ErResult<&str, ModeErr> {
    // Even on options (like .ok_or_else())
    let mode = input.er(ModeErr::missing)?;

    if mode != "haandbold" {
        er_bail!(ModeErr::unknown(mode));
    }

    Ok(mode)
}
```

## Print report

Your `pub fn main()` can return `Result<(), ErReport<AppErr>>`.

In normal code you would usually want to log your report before converting your error to a user facing error.

```rust
if let Err(error) = read_port("fakenumber") {
    eprintln!("{}", error.er_report());
}

// You can also unwrap it (have to pick report/top)
// Or you can use `.unwrap_report()` for short, and `.expect_report("bad port")`
let port = read_port("85").er_report().unwrap();
```

If the caller just wants text:

```rust
let result: Result<u16, String> = read_port("85").er_report_string();
```

`.er_top_string()` does the same with just the outer error, Ok passes through in both. (for public boundaries / actual errors, look at the `Public errors` section).

## Print top error

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

## Helpers / Constructors

The `#[derive(Er)]` gives you `new()` and tuple convenience constructors on structs.

`.er(())` for empty structs, `.er(|_| path)` for 1 field structs, and `.er(|_| (port, enabled))` for 2 or more fields.

Enums need a variant specified like `.er(|| ModeErr::variant_name(arg1))`, or `.er_with(|old| ModeErr::variant_name(old.code))` if you need the old error.

You can still do `.er(|| MyTypeErr { a, b })` but the helpers take care of stuff like not having to call `.into()` or type the error name.

## Use stuff from the previous error

If you need something from the old error, `.er_with()` lets you look at it and still keep it in the tree.

```rust
// The old pal
#[derive(Er)]
pub struct DeviceErr {
    pub code: u8,
}

// Our new one
#[derive(Er)]
pub struct AnalyzeErr {
    pub code: u8,
}

pub fn analyze() -> ErResult<(), AnalyzeErr> {
    // read_device returns ErResult<_, DeviceErr>
    read_device().er_with(|err| err.code)?;
    Ok(())
}
```

If you want to assemble it yourself, use `.er_with(|err| er_built(AnalyzeErr { code: err.code }))`. (This is an annoying case, .er() can do both but .er_with() needs the er_built to help, but just use .er_with() and the constructor helpers).

`.er_with_tree(|t| ...)` gives you the tree if you need to search further down.

Raw errors take the same helpers:

```rust
er_bail!(device.er_with(|err| err.code));
```

## Don't destroy the tree (lose sub errors)

The `.er_with()` above adds to the old tree. Both of these copy the code into a fresh one and throw the old tree away. ALWAYS use `.er()` or `.er_with()` for those cases. NEVER EVER do stuff like this:

```rust
// ! BAD DO NOT DO THIS !
read_device().map_err(|t| ErTree::from(AnalyzeErr::new(t.top.code)))

// ! BAD DO NOT DO THIS !
if let Err(t) = read_device() {
    // the t tree is gone, (womp womp, sad sounds)
    er_bail!(AnalyzeErr::new(t.top.code));
}
```

## Shared / References

If you want the same data in multiple errors without spamming clone you can use `ErShared<T>`.

```rust
use std::{fs, path::PathBuf};

#[derive(Er)]
pub struct OneErr {
    pub path: ErShared<PathBuf>,
}

pub fn read_file(path: PathBuf) -> ErResult<String, OneErr> {
    fs::read_to_string(&path).er(|_| path)
}

#[derive(Er)]
pub struct TwoErr {
    pub path: ErShared<PathBuf>,
    pub machine: String,
}

pub fn read_config(path: PathBuf, machine: &str) -> ErResult<String, TwoErr> {
    read_file(path).er_with(|old| (old.path.clone(), machine))
}
```

## Non errors (ErFormat)

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
let error = ErTree::from(ConnectErr::new(connection));

println!("{}", error.er_top());
```
```text
ConnectErr { connection: Connection { host: "ComputerKatten", port: 85 } }
```

## Collect values

`.er_collect()` keeps the oks and stops at the first error, adding context:

```rust
pub fn read_ports(inputs: &[&str]) -> ErResult<Vec<u16>, ReadPortErr> {
    inputs.iter().map(|input| input.parse()).er_collect(())
}
```

Use `.er_collect_all(())` to keep going and collect every error instead. If anything failed, the collected values get dropped.

## Collect / aggregate / er_all!()

Can be different types of sub error types. Already have an error or tree? Put it in the list directly, no need to wrap it in `Err(...)`.

### Only parent context

```rust
#[derive(Er)]
pub struct ChecksErr {
    pub port: String,
    pub enabled: String,
}

pub fn check_inputs(port: &str, enabled: &str) -> ErResult<(), ChecksErr> {
    // er_all returns (), so Ok(()) here
    er_all!(|_| (port, enabled), [port.parse::<u16>(), enabled.parse::<bool>()])
}
```

### Context on each sub error

Making a type for the suberrors, and giving them context:

```rust
#[derive(Er)]
pub struct SubErr(pub String);

#[derive(Er)]
pub struct ConfigErr {
    pub path: PathBuf,
    pub port: String,
    pub enabled: String,
}

pub fn check_config(path: &Path, port: &str, enabled: &str) -> ErResult<(), ConfigErr> {
    let top_err = |_| (path, port, enabled);
    let host = read_file(path).er(top_err)?;

    er_all!(top_err, [
        port.parse::<u16>().er::<SubErr>(|_| port),
        enabled.parse::<bool>().er::<SubErr>(|_| enabled),
        host.trim().parse::<IpAddr>().er::<SubErr>(|_| host),
    ])
}
```

## Collect / try / get the values / er_try!()

If you need the values `er_try!` gives them back as a tuple when everything succeeds.

```rust
pub fn read_inputs(port: &str, enabled: &str) -> ErResult<(u16, bool), ChecksErr> {
    let (port, enabled) = er_try!(
        |_| (port, enabled),
        [port.parse::<u16>(), enabled.parse::<bool>()],
    )?;

    Ok((port, enabled))
}
```

It runs every result in order and if any fail it drops the successful values and returns one tree with every failure (the top error only gets made if something failed).

For an iterator of the same type you can use `.er_collect_all()`.

## Add errors / er_add!()

You can bunch up errors over time with `.er_add()`, it keeps the same parent and adds more sub errors.

```rust
use std::path::Path;

pub fn read_with_fallback(path: &Path, fallback: &Path) -> ErResult<String, ReadFileErr> {
    let first_error = match read_file(path) {
        Ok(text) => return Ok(text),
        Err(error) => error,
    };

    match read_file(fallback) {
        Ok(text) => Ok(text),
        Err(second_error) => Err(first_error.er_add([second_error])),
    }
}
```

When you want to add multiple errors to a previous one, `er_add!` lets them have different types:

```rust
error = er_add!(error, [first_error, second_error]);
```

It drops the oks from results like `er_all!`.

## Find original error

`.er_find()` also checks `source()`. `.er_find_all()` gets every match.

```rust
if let Err(error) = read_file(Path::new("missing85")) {
    if let Some(source) = error.er_find::<io::Error>() {
        println!("{:?}", source.kind());
    }

    for failed in error.er_find_all::<ReadFileErr>() {
        println!("{}", failed.path.display());
    }
}
```

## Public error

When you (as a library) want to give your consumer an error, don't give them an `ErReport` or an `ErTree`, give them a boring normal error (with `#[derive(Er)]`).

```rust
#[derive(Er)]
pub enum PortError {
    Missing,
    #[er(format = "Not a port: {input}")]
    InvalidPort { input: String },
}

// Consumer doesn't need to have er, it's just a normal error for them.
let error = PortError::invalid_port("fakenumber");
match error {
    PortError::Missing => println!("Where port?"),
    PortError::InvalidPort { input } => println!("Try 85, not {input}"),
}
```

Use `.er_into()` to save the report on your type, you get the tree first if you want to print it:

```rust
#[derive(Er)]
#[er(format = "{report}")]
pub struct ApiError {
    #[er(into_top)]
    pub kind: ReadPortErr,

    #[er(into_report_string)]
    pub report: String,

    pub request_id: u32,
}
pub fn public_read_port(input: &str, request_id: u32) -> Result<u16, ApiError> {
    read_port(input).er_into(|tree| {
        eprintln!("{}", tree.er_report());

        request_id
    })
}
```

The closure only runs on errors and returns your other fields, one value or a tuple in field order. With no other fields you can use `.er_into(|_| {})`. You can swap the `String` for `ErSnapshot` with `#[er(into_snapshot)]` if you need the structure.

You should still print / show your error in your applications of course, and I'll even argue that as a library, if you at least give a string report to your consumer, you're gonna have great bug reports from users and a much easier time fixing bugs. Printing a report is often times more valuable than a friendly message you made up. Don't destroy your type. Include the message in the type itself next to the other data instead.

Yes this opts out from auto implementing stuff with `From<>`, this is on purpose so you have to go through `.er_into()` and decide what to do with the report.

## Non errors (values)

Never do this on a tree, you nuke it! This makes a new `ErTree`.

For values that don't implement `Error` like `Err(85)`, put the data in your own error:

```rust
#[derive(Er)]
pub struct DeviceErr {
    pub status: u8,
}
pub fn check_device(result: Result<(), u8>) -> ErResult<(), DeviceErr> {
    // Remember, in rust if the first value of a closure just gets passed to a function,
    // you can pass the function directly. So you could also do `result.er_val(DeviceErr::new)`
    result.er_val(|status| DeviceErr::new(status))
}
```

If you have some data, you want some logic to convert, i would NOT put it a layer beneath (by mapping it to an error, then putting it below).

Use types and data, here's an example with `JsValue` where we just use it as data on our error. 

```rust
use js_sys::Reflect;
use wasm_bindgen::JsValue;

// First we make our own type, remember this is NOT an error
#[derive(ErFormat)]
pub struct JsData {
    pub message: String,
    pub stack: Option<String>,
}
impl From<JsValue> for JsData {
    fn from(value: JsValue) -> Self {
        let text = |field: &str| {
            Reflect::get(&value, &field.into())
                .ok()
                .and_then(|value| value.as_string())
        };

        Self {
            message: value
                .as_string()
                .or_else(|| text("message"))
                .unwrap_or_else(|| format!("{value:?}")),
            stack: text("stack"),
        }
    }
}

// Now we make the error and just include the data
#[derive(Er)]
pub struct ConnectErr {
    pub server: String,
    pub details: JsData,
}
pub fn connect(server: &str, result: Result<(), JsValue>) -> ErResult<(), ConnectErr> {
    let e = |value| ConnectErr::new(server, value);
    result.er_val(e)
}
```

The constructor does the `.into()` for us. And if we had variants that did not have the data, i would make an enum and only have the data on the some of the variants.

## Bail

You can use `er_bail!(err)` if you don't want to type `return Err(ErTree::from(err))` (You can give it an existing tree too).

```rust
if path.is_empty() {
    er_bail!(|_| (path, "no file given"));
}
```

## Snapshots

Saves msgs and the tree structure for special occasions (as strings, not the error types).

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

Can still print `.er_top()` or `.er_report()`. Each entry has its depth and the index of the error above it.

On a Result, `.er_snapshot()` saves Err and leaves Ok alone (for public boundaries / actual errors, look at the `Public errors` section).

```rust
let result: Result<u16, ErSnapshot> = read_port("fakenumber").er_snapshot();
```

Enable `serde` on Er, then add `serde_json` (or toml, or whatever).

```toml
[dependencies]
er = { version = "0.7", features = ["serde"] }
serde_json = "1"
```

```rust
let snapshot = read_port("fakenumber").unwrap_err().er_snapshot();
let json = serde_json::to_string_pretty(&snapshot).unwrap();
std::fs::write("/tmp/error.json", &json).unwrap();
```

## Tests

`test` is on by default. If you only use Er in tests, put the dependency here:

```toml
[dev-dependencies]
er = "0.7"
```

Make the test return `ErTest` and use `?`. Ordinary errors and Er trees get `ErTestError` on top with the location of the `?`, and keep the original errors below it. It prints as `TestError`.

```rust,ignore
use er::*;

#[derive(Er)]
pub struct ReadPortErr;

pub fn read_port(input: &str) -> ErResult<u16, ReadPortErr> {
    input.parse().er(())
}

#[test]
pub fn the_best_test() -> ErTest {
    read_port("nope")?;
    Ok(())
}
```
```text
Error: TestError @ tests/the_best_test.rs:12:5
`- ReadPortErr @ tests/the_best_test.rs:7:19
   `- invalid digit found in string
test the_best_test ... FAILED
```

Make your test helper functions return other types since `?` would fall through without adding context.

For Options, use `option.er_test()?`.

The macros work too, but you do need to use `er_all!(|| ErTestError, results)?` so Rust knows which error to build.

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

pub fn request(input: &str) -> ErResult<u16, RequestErr> {
    handler(input).er(())
}
```

[Wrap options and the trait impl](macros.md#wrap).

If the foreign trait needs the Wrap itself to implement `Error`, use [std_error](macros.md#wrap-with-std_error) and `.er_wrap()` when adding context, so you can still find the errors inside.

## Sharing a result multiple times

If several callers need the same result, give them a type which has what they need: 

```rust
#[derive(Er)]
pub enum SessionErrKind {
    Failed { context: String },
    CleanupFailed,
}

#[derive(Er)]
// Since debug gives us literal '\n'
#[er(format = "{report}")]
pub struct SessionError {
    #[er(into_top)]
    pub kind: SessionErrKind,

    #[er(into_report_string)]
    pub report: String,
}

let completion: Result<(), Arc<SessionError>> = run_session().er_into(|tree| {
    // It automatically does the .into() for the Arc too (also works without it).
    eprintln!("{}", tree.er_report());
});

let reader = completion.clone();

// Somewhere else
if let Err(error) = reader {
    match error.kind {
        SessionErrKind::Failed { .. } => eprintln!("session said nope: {error}"),
        SessionErrKind::CleanupFailed => eprintln!("cleanup gave up: {error}"),
    }
}
```

## Opaque (edge case)

Look at the `Sharing a result`(right above) and the `Public error`(further up) examples first. That's for when you want to make a public type (attaching a string report, or a snapshot).

But if you want to force the real report, into an Error, there is the escape hatch: 

```rust
pub fn run() -> anyhow::Result<()> {
    // Or `.er_top()`
    read_port("nope").er_report().opaque_err()?;
    Ok(())
}
```

Nothing is deleted `ErAsError` keeps the presentation in its public `.0` field.

But `.opaque_err()` stops searches (source() is empty), and going the other way, Anyhow's boxed conversion can also be sneaky and hide types from er_find. [The anyhow example](../../integrations/anyhow/src/lib.rs) shows both.

## Stack traces

Enable `stack_traces` (needs `std`), then add `.er_trace()` where you want to capture the stack (only runs on errors).

```rust
pub fn read_port(input: &str) -> ErResult<u16, ReadPortErr> {
    input.parse().er(()).er_trace()
}

if let Err(error) = read_port("fakenumber") {
    eprintln!("{}", error.er_report());
    for trace in &error.stack_traces {
        eprintln!("{trace}");
    }
}
```

They follow the tree as you add context or aggregate it. You print them separately, you can always call `.er_trace()` again and there's no env variables to turn on.

If you need the error, you can use the index with `error.er_at_index(trace.error_index)`.

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

## Tricky example (combination)

Foreign errors, own errors, the original error, string context, typed context, using / consuming, public boundary

```rust
#[derive(Er)]
pub enum ListenErr {
    InvalidInput { input: String },
    SacredPort,
    BindFailed { address: SocketAddrV4, kind: io::ErrorKind, available_ports: Vec<u16> },
}

pub fn listen(input: &str) -> ErResult<TcpListener, ListenErr> {
    let (ip, port) = input.split_once(':').unwrap_or((input, ""));

    // First 2 errors, to us they're both just bad input
    let ip = ip.parse::<Ipv4Addr>().er(|| ListenErr::invalid_input(input))?;
    let port = port.parse::<u16>().er(|| ListenErr::invalid_input(input))?;

    // Third error, our own rule that port 85 is sacred
    if port == 85 {
        er_bail!(ListenErr::sacred_port());
    }

    // Fourth error, we might want to match on what happened
    let address = SocketAddrV4::new(ip, port);
    TcpListener::bind(address).er_with(|err| {
        let available_ports = find_available_ports(address);
        ListenErr::bind_failed(address, err.kind(), available_ports)
    })
}

// Someone using our public API doesn't need Er.
// I use 'Err' for internal errors, and 'Error' for public facing ones.
#[derive(Er)]
#[er(format = "{report}")]
pub struct ListenError {
    #[er(into_top)]
    pub kind: ListenErr,

    #[er(into_report_string)]
    pub report: String,
}
pub fn public_error_example(input: &str) -> Result<TcpListener, ListenError> {
    listen(input).er_into(|tree| {
        eprintln!("{}", tree.er_report());
    })
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
