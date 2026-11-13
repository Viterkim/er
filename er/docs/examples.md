# Examples / Patterns

All examples use `use er::*;`. Start with the fields you already have. `.er(())` builds an empty struct, `.er(path)` fills one field, and `.er((port, enabled))` fills two. For an enum, pick the variant.

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
    fs::read_to_string(path).er(path)
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

```rust
if let Err(error) = read_port("fakenumber") {
    eprintln!("{}", error.er_report());
}

let port = read_port("85").er_unwrap_report();
```

Make the text in the `Err` arm so you can still match on the typed error or add another layer. `error.er_report()` gives you the whole report as a String. Call `.er_report()` on the Result when all you need is text, and you get `Result<u16, String>`. `.er_unwrap_report()` and `.er_expect_report("bad port")` panic with the report.

```rust
match read_port(input) {
    Ok(port) => use_port(port),
    Err(error) => tracing::warn!(report = %error.er_report(), "bad port"),
}
```

For a logger that wants each line on its own, `error.for_each_line(|line| log_line(line))?`, or `try_for_each_line` when writing a line can fail. For a preview, `error.er_write_report(&mut preview)` writes straight to your `fmt::Write` buffer. A buffer can stop when it's full or keep reading to save the cause at the end.

Return `ErMain` from main and use `?` on ordinary errors and Er trees. A failure prints the whole report.

```rust
pub fn main() -> ErMain {
    let port = read_port("85")?;
    serve(port)?;

    Ok(())
}
```

Use `aktor_start(setup).await.er_test()?` to keep a library's `(Error, String)` together when main prints it.

## Print top error

```rust
if let Err(error) = read_file(Path::new("missing85")) {
    eprintln!("{}", error.top);
    println!("Missing: {}", error.top.path.display());
}
```

```text
ReadFileErr { path: "missing85" }
Missing: missing85
```

`error.top` is your own error, print it or read its fields.

## Helpers / Constructors

The return type already tells us which error we're making. Give `.er()` its fields in order and let it do the conversions when something fails:

```rust
#[derive(Er)]
pub struct ReadConfigErr {
    pub path: PathBuf,
    pub action: String,
}
pub fn read_config(path: &Path) -> ErResult<String, ReadConfigErr> {
    fs::read_to_string(path).er((path, "load config"))
}
```

The path reference is cheap to pass. Er converts it when the read fails. Put a lookup like `user.name()` in a closure, `.er(|_| (path, user.name()))`, so the successful read skips it too.

You still get `new(...)` for constructing a value directly, and `.er(|| MyTypeErr { a, b })` works for a finished error.

Pass values directly, there's usually no need for `.as_ref()` or `.as_str()`. You can pass `T` straight into an `Option<T>` field that uses the standard conversion.

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

Return a complete error when you need to pick a variant or name the fields, like `.er_with(|err| AnalyzeErr { code: err.code })`.

The old error stays in the tree, so the closure borrows it. To carry an owned name up too, use `.er_with(|err| (err.name.clone(), path))`. On an `ErShared` field that only clones the handle.

Raw errors take the same helpers:

```rust
er_bail!(device.er_with(|err| err.code));
```

Never rebuild the error with `map_err`. The old tree is where everything below the code lives. `.er_with()` keeps it there.

## Shared / References

Sometimes a caller far up needs to react to something way down, like which limit an upload hit. Put it on each layer as an `ErShared<T>` and they all point at the same value, no cloning it 5 times on the way up:

```rust
#[derive(Debug)]
pub enum Limit {
    Bytes { used: u64, max: u64 },
    Streams { open: u32, max: u32 },
}

#[derive(Er)]
pub struct ReserveErr {
    pub limit: ErShared<Limit>,
}

pub fn reserve(pool: &Pool, bytes: u64) -> ErResult<(), ReserveErr> {
    match pool.check(bytes) {
        Some(limit) => er_bail!(|_| limit),
        None => Ok(()),
    }
}

#[derive(Er)]
pub struct UploadErr {
    pub file: String,
    pub limit: ErShared<Limit>,
}

pub fn upload(pool: &Pool, file: &str) -> ErResult<(), UploadErr> {
    reserve(pool, 85).er_with(|err| (file, err.limit.clone()))
}

// The caller way up
if let Err(error) = upload(&pool, "cat.png") {
    match &*error.top.limit {
        Limit::Bytes { .. } => wait_for_space(),
        Limit::Streams { .. } => close_idle_streams(),
    }
}
```

That `.clone()` is cheap, everyone shares the same value. Make it where the error happens, like the `er_bail!` above, then it only allocates when something fails.

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
let error = er_new!(ConnectErr::new(connection));

println!("{}", error.top);
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

`er_all!` returns `Ok(())` if every check passed, including when the list is empty. Different error types are fine. Already have an error or tree? Put it in the list directly, no need for `Err(...)`.

### Only parent context

```rust
#[derive(Er)]
pub struct ChecksErr {
    pub port: String,
    pub enabled: String,
}

pub fn check_inputs(port: &str, enabled: &str) -> ErResult<(), ChecksErr> {
    er_all!((port, enabled), [port.parse::<u16>(), enabled.parse::<bool>()])
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
    let top_err = (path, port, enabled);
    let host = read_file(path).er(top_err)?;

    er_all!(top_err, [
        port.parse::<u16>().er::<SubErr>(port),
        enabled.parse::<bool>().er::<SubErr>(enabled),
        host.trim().parse::<IpAddr>().er::<SubErr>(&host),
    ])
}
```

## Collect / try / get the values / er_try!()

If you need the values `er_try!` gives them back as a tuple when everything succeeds.

```rust
pub fn read_inputs(port: &str, enabled: &str) -> ErResult<(u16, bool), ChecksErr> {
    let (port, enabled) = er_try!(
        (port, enabled),
        [port.parse::<u16>(), enabled.parse::<bool>()],
    )?;

    Ok((port, enabled))
}
```

It runs them in order and collects every failure, dropping the oks if any failed.

For an iterator of the same type you can use `.er_collect_all()`.

## Add errors / er_add!()

If the first file failed and the fallback fails too, keep the first tree and add the second error below it.

```rust
use std::path::Path;

pub fn read_with_fallback(path: &Path, fallback: &Path) -> ErResult<String, ReadFileErr> {
    let first_error = match read_file(path) {
        Ok(text) => return Ok(text),
        Err(error) => error,
    };

    match read_file(fallback) {
        Ok(text) => Ok(text),
        Err(second_error) => Err(first_error.er_add(second_error)),
    }
}
```

Once an import has failed, even a successful rollback leaves you with an import error. Start with just the top, then add whatever goes wrong during cleanup.

```rust
let error = er_new!(ImportErr::rejected(path));
let error = error.er_add(rollback_import());

er_bail!(error);
```

When you already have the failures, pass them as the second argument to `er_new!`. To add different types to an existing tree, use `er_add!(error, [rollback_import(), close_file()])`. You can move a field straight into a new tree as well, like `let error: ErTree<FileErr> = er_new!(path);`.

## Find original error

A diagnostic can look through the tree for every file that failed. If finding one changes what your app does next, put that case on the top error and carry it up with `.er_with()`.

`.er_find()` also checks `source()`. `.er_find_all()` gets every match, and `.er_contains()` when you only want to know it's in there.

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

Inside your code, keep the tree. Someone using your library only needs your error type and whatever they should do with it. When they also have to log the failure, send the whole report next to it.

### With its report

```rust
#[derive(Er)]
pub enum PortError {
    InvalidPort { input: String },
}

pub fn read_port(input: &str) -> Result<u16, (PortError, String)> {
    input.parse().er(|| PortError::invalid_port(input)).er_split()
}
```

```rust
if let Err((PortError::InvalidPort { input }, report)) = read_port("nope") {
    eprintln!("Fix the port in your config: {input}\n{report}");
}
```

`PortError` and `String` work with any Er version or no Er at all. On Er 0.9, a plain `.er()` adds the error to the caller's tree and saves the report. Keep the report next to the library error. Other reporters can squash a report hidden inside the error into a line or miss it entirely.

### Send a report

When the message itself needs the report, give it a field and move the report String into it.

```rust
#[derive(serde::Serialize)]
pub struct PortFailure {
    pub report: String,
}

let port = input.parse::<u16>().er(|| PortError::invalid_port(input));
let reply = port.er_split_with(|(_, report)| PortFailure { report });
```

Give the message a typed field too if the other end needs to decide what to do next.

### Without a report

For an HTTP API, the report goes in your logs. The client gets the error it can use.

```rust
async fn create_user(Json(user): Json<NewUser>) -> Result<Json<User>, UserError> {
    let user = insert_user(user).await.er_split_with(|(error, report)| {
        tracing::error!("{report}");
        error
    })?;

    Ok(Json(user))
}
```

### Errors from your callbacks

When all your library does with a callback failure is report it, ask for `Result<(), String>`. Your user's error types stay theirs.

```rust
// Inside the library, on a failure the callback's report goes in your tree.
check(&row).er(|| ImportErr::invalid_row(row.number))?;

// A user with Er can pass you their report.
import(path, |row| validate(row).er_report()).er(())?;
```

A callback that can tell you to skip a row or stop should return an enum you can match on.

## Reports from somewhere else

Aktor gives you `(AktorStartError, String)`. Add what you were starting and Er saves Aktor's report under it.

```rust
let started = aktor_start(setup).await.er(|| AudioErr::stopped(name))?;
```

```text
AudioErr::Stopped { name: "speaker" } @ src/audio.rs:88:31
`- actor startup failed

---- Saved: aktor::setup::AktorStartError ----
actor startup failed for speaker
OpenErr { path: "app.db" } @ src/device.rs:39:10
`- disk full
```

A library can use `Option<String>` when it only has a report for some failures. Er saves it when it's there.

When the report sits somewhere else, like a field on their error or a server's response body, save it yourself where it comes in. Say what it is in the title so whoever reads the report isn't guessing:

```rust
client.send(request).er_save("server", |err| &err.body).er(id)?;
```

From a Result, borrow the report or snapshot out of the error. You can save directly on a new error too, `er_bail!(SyncErr::server(code).er_save("server", report))`.

## Non errors (values)

Never use `.er_val()` on a tree or a library's `(Error, String)` reply, you nuke the report! Use `.er_with()` when you need something from the old error.

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

Use `er_bail!(err)` to return an error now. You can give it an existing tree or pass sub errors as the second argument, just like `er_new!`.

```rust
if path.is_empty() {
    er_bail!((path, "no file given"));
}
```

## Snapshots

Saves messages and the tree structure for Christmas or other special occasions. You can drop the error and still print the report or walk its entries.

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

`snapshot.top()` is the top message and `.er_report()` the whole report. Each entry has its depth and the index of the error above it.

On a Result, `.er_snapshot()` saves Err and leaves Ok alone.

```rust
let result: Result<u16, ErSnapshot> = read_port("fakenumber").er_snapshot();
```

The saved result still has `.er_report()` and `.er_unwrap_report()`. Send the report as text, or unpack `let ErSnapshot { entries, saved } = snapshot;` for your own format. The entries give you the tree. Anything attached with `.er_save()` is in `saved`, including snapshots and backtraces.

Saving one is for a failure that explains the new one without being its cause, like a sync that failed earlier and left you offline. It prints as its own block below, so the old tree never ends up inside your error:

```rust
let last_sync = sync(&db).await.er_snapshot().err();

reconnect(&server).er(|_| ReconnectErr.er_save("last sync", &last_sync))?;
```

```text
ReconnectErr @ src/net.rs:21:17
`- connection refused

---- Saved: last sync ----
SyncErr @ src/sync.rs:10:24
`- database is locked
```

When the old failure is the cause, keep it typed with `ErSharedTree` and put it below your error with `(&failure).er(..)`.

## Tests

ErTest is always available. If you only use Er in tests, put it in dev-dependencies.

```toml
[dev-dependencies]
er = "0.9"
```

Make the test return `ErTest` and use `?`. Ordinary errors and Er trees get `ErFailed` on top with the location of the `?`, and keep the original errors below it. It prints as `Failed`, `main` returning `ErMain` works the same way.

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
Error: Failed @ tests/the_best_test.rs:12:5
`- ReadPortErr @ tests/the_best_test.rs:7:19
   `- invalid digit found in string
test the_best_test ... FAILED
```

Give test helpers their own errors. A helper returning `ErTest` already has the generic `Failed` error, so `?` just passes it through.

For Options, unit errors (`Result<T, ()>`) and an error with its report next to it, `(Error, String)`, use `.er_test()?`.

When a test expects a failure, say which error it should be. `er_assert_err!` fails on Ok and on any other error, and prints the whole report so you see what you got instead:

```rust
#[test]
pub fn rejects_words() -> ErTest {
    er_assert_err!(read_port("nope"), ReadPortErr);
    Ok(())
}
```

If you want to inspect the error yourself, `let error = er_assert_err!(load(path));` hands it back. An unexpected Ok fails the test, even if the value doesn't implement `Debug`.

## Other traits

axum's `IntoResponse` doesn't need Wrap. Put it on your own plain error and split in the handler:

```rust
async fn create_user_handler(State(db): State<Db>, Json(user): Json<NewUser>) -> Result<Json<User>, UserError> {
    create_user(&db, user).await.map(Json).er_split_with(|(error, report)| {
        tracing::error!("{report}");
        error
    })
}
```

When the trait needs the whole tree, put it on a Wrap you own.

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

[Wrap options and the trait impl](https://github.com/Viterkim/er/blob/main/er/docs/macros.md#wrap).

If the foreign trait needs the Wrap itself to implement `Error`, use [std_error](https://github.com/Viterkim/er/blob/main/er/docs/macros.md#wrap-with-std_error) and `.er_wrap()` when adding context, so you can still find the errors inside.

## Sharing a result multiple times

Inside your own code, `.er_share()` the failure so everyone can put it below their own error. `opened.as_ref()` lets playback and capture borrow the same speaker. Everyone gets the whole tree in their report (and `er_find` still finds stuff inside it).

```rust
pub struct Device {
    pub opened: Result<Speaker, ErSharedTree<OpenErr>>,
}

impl Device {
    pub fn open(&mut self) {
        self.opened = open_speaker().er_share();
    }

    pub fn playback(&self) -> ErResult<&Speaker, PlaybackErr> {
        self.opened.as_ref().er(())
    }

    pub fn capture(&self) -> ErResult<&Speaker, CaptureErr> {
        self.opened.as_ref().er_with(|err| err.device.clone())
    }
}
```

```text
PlaybackErr @ src/device.rs:14:29
`- OpenErr { device: "speaker" } @ src/device.rs:3:12
   `- DriverErr { code: 85 } @ src/device.rs:2:47
```

`failure.top`, `.er_report()` and friends work just like on a normal tree. You can't change a shared tree though, it's frozen. To add more later, make a new tree on top and swap that in, `er_new!(SessionErr, [&failure, cleanup_error]).er_share()`. When cleanup hands you the same failure again, `tree.er_contains_shared(&failure)` tells you if it's already in there.

A new error can use `.er_share()` directly too, `MyErr::denied(reason).er_share()`.
So can a library's `Result<T, (Error, String)>`. The shared tree keeps the returned error on top and saves its report.

If it leaves your crate, hand out your error with the report next to it. The pair clones whenever your error does:

```rust
#[derive(Clone, Er)]
pub enum SessionError {
    Failed { context: String },
    CleanupFailed,
}

let completion: Result<(), (SessionError, String)> = run_session().er_split();

let reader = completion.clone();

// Somewhere else
if let Err((error, report)) = reader {
    match error {
        SessionError::Failed { .. } => eprintln!("session said nope:\n{report}"),
        SessionError::CleanupFailed => eprintln!("cleanup gave up:\n{report}"),
    }
}
```

## Foreign callbacks

SQLite chooses the error type for `FromSql`, so this callback can't return our tree. It gives us a box for an ordinary error. We put the invalid value and its location there:

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

`new(value)` creates the ordinary error and captures the line where we call it inside the closure. The location isn't a constructor argument.

The query's caller gets to choose its error type. This is where we use `.er()` and say which item we were reading:

```rust
#[derive(Er)]
pub struct ReadKindErr(pub i64);
pub fn read_kind(db: &Connection, id: i64) -> ErResult<ValueKind, ReadKindErr> {
    db.query_row("SELECT kind FROM items WHERE id = ?1", [id], |row| row.get(0))
        .er(id)
}
```

The callback supplies the diagnostic, and this call builds the tree around SQLite's error. The return types tell us which one we're writing.

## Stack traces

Enable `stack_traces` (needs `std`), then save a backtrace where you want one. The closure only runs if it failed, so a working call never captures:

```rust
use std::backtrace::Backtrace;

pub fn read_port(input: &str) -> ErResult<u16, ReadPortErr> {
    input.parse().er(|_| ReadPortErr.er_save("stack trace", Backtrace::force_capture))
}

if let Err(error) = read_port("fakenumber") {
    eprintln!("{}", error.er_report());
}
```

It prints below the report with everything else you saved, wherever that report ends up. No env variables to turn on.

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

Use `format` for both outputs, `debug` for Debug alone and `display_alt` for `{err:#}`. You can also skip fields, censor them or use your own formatting function. [Macro options](https://github.com/Viterkim/er/blob/main/er/docs/macros.md)

## Tricky example (combination)

Someone calling `listen` cares whether to fix their input or try another port. We can give them those cases and still keep the parse or socket error that got us there.

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

// Someone using our public API doesn't need Er, they match on ListenErr and print the report.
pub fn public_error_example(input: &str) -> Result<TcpListener, (ListenErr, String)> {
    listen(input).er_split()
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

[See how the other error libraries handle this example](https://github.com/Viterkim/er/blob/main/er/docs/tricky-error-comparison.md).
