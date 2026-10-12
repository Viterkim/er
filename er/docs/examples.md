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

`.er_top_string()` gives just the outer error, Ok passes through in both.

You can log a failure and still return it. The log gets one line, and the caller gets the full report:

```rust
pub fn read_and_log(input: &str) -> Result<u16, ErReport<ReadPortErr>> {
    let result = read_port(input);

    if let Err(err) = &result {
        eprintln!("{}", err.er_report().single_line());
    }

    result.er_report()
}
```

On the tree itself, `.er_report()` borrows it and `.into_er_report()` moves it into the report. The top helpers work the same way. For a logger that wants each line separately, use `report.for_each_line(|line| log_line(line))?`, or `try_for_each_line` when writing a line can fail.

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

The return type already tells us which error we're making. Give `.er()` its fields in order and let it do the conversions when something fails:

```rust
#[derive(Er)]
pub struct ReadConfigErr {
    pub path: PathBuf,
    pub action: String,
}
pub fn read_config(path: &Path) -> ErResult<String, ReadConfigErr> {
    fs::read_to_string(path).er(|_| (path, "load config"))
}
```

No `ReadConfigErr::new(...)`, `path.to_path_buf()` or `String::from(...)` at the call. The same field helpers work with `.er_with()`, `.er_val()` and `.er_into()`, each giving the closure the thing you need to build from. To stop a conversion on a particular field, mark it with `#[er(exact)]` and pass its actual type.

You still get `new(...)` for constructing a value directly, and `.er(|| MyTypeErr { a, b })` works for a finished error. Use `#[er(no_constructors)]` to turn off the constructors and field helpers when you want to handle construction yourself.

Each function's return type usually gives Rust everything it needs. An extra intermediate error in a chain can need its name, like `.er::<ReadConfigErr>(|_| (path, "load config"))`. Put the relevant fields on the error your function returns and you usually won't need that extra layer.

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

For a finished struct or a foreign error, `.er_with()` needs `er_built(...)`, like `.er_with(|err| er_built(AnalyzeErr { code: err.code }))`.

Derived enums work directly even with `{ ... }`, though `ModeErr::unknown(...)` saves you doing the field conversions yourself.

The typed top isn't always the error with the data you need. `.er_with_tree()` gives you the whole tree to search, while keeping it below the new context:

```rust
#[derive(Er)]
pub struct AnalyzeBatchErr {
    pub device_codes: Vec<u8>,
}
pub fn analyze_batch(result: ErResult<(), ChecksErr>) -> ErResult<(), AnalyzeBatchErr> {
    result.er_with_tree(|tree| {
        tree.er_find_all::<DeviceErr>()
            .map(|err| err.code)
            .collect::<Vec<_>>()
    })
}
```

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

Use `ErShared<T>` to keep the same data in multiple errors without copying it:

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
    read_file(path).er_with(|err| (err.path.clone(), machine))
}
```

That `.clone()` only copies the shared handle, the path stays in the same allocation. The old error stays in the tree with its handle too.

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

Different error types are fine here. Already have an error or tree? Put it in the list directly, no need for `Err(...)`.

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

It runs them in order and collects every failure, dropping the oks if any failed.

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

You should still print / show your error in your applications of course, and I'll even argue that as a library, if you at least give a string report to your consumer, you're gonna have great bug reports from users and a much easier time fixing bugs. Printing a report is often times more valuable than a friendly message you made up. Don't destroy your type. Include the message in the type itself next to the other data instead.

Use `.er_into()` to save the report beside the error, you can also use the convenience macros to help out.

```rust
#[derive(Er)]
#[er(format = "read port failed", display_alt = "{report}")]
pub struct ApiError<T = ()> {
    #[er(into_top)]
    pub kind: ReadPortErr,

    #[er(into_report_string)]
    pub report: String,

    #[er(default)]
    pub data: T,
}
pub fn public_read_port(input: &str) -> Result<u16, ApiError> {
    read_port(input).er_into(|report| report)
}
```

The consumer gets a normal error, with the report available through `{err:#}` or the field itself. Ordinary Display and Debug stay small, so someone using another error library won't get a giant report dumped into the middle of theirs.

`data` defaults to `()` here. You can give them a payload too, without requiring it to implement `Default`:

```rust
pub fn public_read_port_with_data<T>(input: &str, data: T) -> Result<u16, ApiError<T>> {
    read_port(input).er_into(|report| (report, data))
}
```

For your own fallback, put a Rust expression on the field, like `#[er(default = make_data())]`. Supplying the payload still uses the value you gave it.

You can also choose the fields in the closure yourself, just like `.er_val()`. A diagnostics-only type can take `.er_into(|report| report.er_report_string())`. With the typed top too, you can print the report before moving it out:

```rust
pub fn public_read_port(input: &str) -> Result<u16, ApiError> {
    read_port(input).er_into(|report| {
        let text = report.er_report_string();
        eprintln!("{text}");

        (report.tree.top, text)
    })
}
```

## Non errors (values)

Never do this on a tree, you nuke it! This makes a new `ErTree`.

For values that don't implement `Error` like `Err(85)`, put the data in your own error:

```rust
#[derive(Er)]
pub struct DeviceErr {
    pub status: u8,
}
pub fn check_device(result: Result<(), u8>) -> ErResult<(), DeviceErr> {
    result.er_val(|v| v)
}
```

Put the value on the error this function returns. Give the field its own data type when you need conversion logic, and let the tuple helpers call `.into()` for you. You don't need to turn the value into another error and add a layer above it.

Here's `JsValue` as data on our error:

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
    result.er_val(|v| (server, v))
}
```

The function's return type chooses `ConnectErr`, and the helpers convert `server` into `String` and `v` into `JsData`. If some cases don't have that data, use an enum and put it on the variants that do.

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

A type you own can save one with an `ErSnapshot` field marked `#[er(into_snapshot)]`, then `.er_into(|report| report)`. That field is still an Er type. For a public error without Er types, serialize the snapshot and store the resulting text instead.

On a Result, `.er_snapshot()` saves Err and leaves Ok alone.

```rust
let result: Result<u16, ErSnapshot> = read_port("fakenumber").er_snapshot();
```

The saved result keeps the printing and unwrap helpers, so `result.unwrap_report()` prints the saved report if it fails. To show it and keep it around, borrow it with `result.as_ref().er_report()` or use `snapshot.er_report().single_line()` for one log line. `snapshot.into_er_report()` gives you an owned report, just like a live tree.

Enable `serde` on Er, then add `serde_json` (or toml, or whatever).

```toml
[dependencies]
er = { version = "0.8", features = ["serde"] }
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
er = "0.8"
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

For Options and unit errors (`Result<T, ()>`), use `.er_test()?`.

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
#[er(format = "session failed", display_alt = "{report}")]
pub struct SessionError {
    #[er(into_top)]
    pub kind: SessionErrKind,
    #[er(into_report_string)]
    pub report: String,
}

let completion: Result<(), Arc<SessionError>> = run_session().er_into(|report| {
    let text = report.er_report_string();
    eprintln!("{text}");

    report
});

let reader = completion.clone();

// Somewhere else
if let Err(error) = reader {
    match error.kind {
        SessionErrKind::Failed { .. } => eprintln!("session said nope: {error:#}"),
        SessionErrKind::CleanupFailed => eprintln!("cleanup gave up: {error:#}"),
    }
}
```

## Foreign callbacks

Sometimes another library makes you return its error type. SQLite's `FromSql` does that, with a box for your own diagnostic. We only need to say which value was invalid and where we found it:

```rust
#[derive(Er)]
#[er(format = "invalid value kind {value:?} @ {location}")]
pub struct ValueKindErr {
    pub value: String,
    #[er(src_location)]
    pub location: ErSrcLocation,
}

impl FromSql for ValueKind {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        let value = value.as_str()?;

        Self::from_name(value)
            .ok_or_else(|| FromSqlError::other(ValueKindErr::new(value)))
    }
}
```

`new(value)` fills the location with its call site. The box carries one ordinary diagnostic. Handle the database operation with `.er()` where you call it, so that layer says what you were reading.

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

## Formatting your types

The default output shows your type and its data. You can give Display a different message while Debug keeps that view:

```rust
#[derive(Er)]
#[er(display = "Couldn't connect to {host} on port {port}")]
pub struct ConnectErr {
    pub host: String,
    pub port: u16,
}

let err = ConnectErr::new("ComputerKatten", 85);
println!("{err}");
println!("{err:?}");
```

```text
Couldn't connect to ComputerKatten on port 85
ConnectErr { host: "ComputerKatten", port: 85 }
```

Use `format` to give both the same text, or `debug` to change only Debug. They work on structs, enums and individual variants. The public error example uses `display_alt` so `{err:#}` can show a saved report while ordinary output stays small.

Templates use Display for `{field}` and Debug for `{field:?}`, with `{0}` for tuple fields. You can reach into the data with `{details.message}` or `{oldest.0}` too. To leave a field out of the default output, use `#[er(skip)]`, or `#[er(censor)]` to print `*CENSORED*` while keeping its data.

For formatting that needs actual logic, give the setting a function instead. It gets your value and the formatter, so conditions stay normal Rust:

```rust
#[derive(ErFormat)]
#[er(display = Self::summary)]
pub struct Window {
    pub first: u64,
    pub latest: u64,
}
impl Window {
    pub fn summary(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.first == self.latest {
            write!(f, "revision {}", self.first)
        } else {
            write!(f, "revisions {} to {}", self.first, self.latest)
        }
    }
}
```

The same function form works with `format`, `debug` and `display_alt`.

[The generated code and macro options](macros.md).

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
#[er(format = "listen failed", display_alt = "{report}")]
pub struct ListenError {
    pub kind: ListenErr,
    pub report: String,
}
pub fn public_error_example(input: &str) -> Result<TcpListener, ListenError> {
    listen(input).er_into(|report| {
        let text = report.er_report_string();
        eprintln!("{text}");

        (report.tree.top, text)
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
