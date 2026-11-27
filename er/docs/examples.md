# Examples / Patterns

All examples use `use er::*;`

`#[derive(Er)]` implements `Error`, `Display`, `Debug` and helpers.

## TLDR

Structs: Use `.er(())` for empty structs, `.er(path)` for 1 field structs, and `.er((port, enabled))` for 2 or more fields (closures for non `Copy + Debug` values).

Enums: variant like `.er(|| ModeErr::variant_name(arg1))`.

Give each fallible function its own error type so `?` can't silently pass the same error all the way up without adding context. 

Keep the cases your caller cares about! (the tree keeps the original errors underneath).

Don't specify the type everywhere, inference works.

On things like Option<> you don't have to use Some(v)/.into().

```rust
// Put your error type directly above your function.
// You shouldn't use '#[er(format)]'
#[derive(Er)]
pub struct MyFuncErr;

pub fn my_func() -> ErResult<(), MyFuncErr> {
    something_else().er(())
}

#[derive(Er)]
pub struct MyOtherErr {
    // Keep your data away from your msg (we have types in rust, don't pack stuff into a String)
    pub path: PathBuf, 
    pub msg: &'static str,
}

pub fn other_func(path: &str) -> ErResult<(), MyOtherErr> {
    // Values directly, no .as_ref()/.as_str(), and Option<T> without Some().
    if path.is_empty() {
        er_bail!((path, "no file given"));
    }
    my_func().er((path, "some cool msg"))
}
```

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

## Use the previous error

If you need something from the old error, `.er_with()` lets you look at it and still keep it in the tree.

```rust
#[derive(Er)]
pub struct DeviceErr {
    pub code: u8,
}

// Our new one, very realistic example
#[derive(Er)]
pub struct AnalyzeErr {
    pub code: u8,
}

pub fn analyze() -> ErResult<(), AnalyzeErr> {
    // We assume read_device() returns ErResult<_, DeviceErr>
    read_device().er_with(|err| err.code)?;
    Ok(())
}
```

If it's non Copy (`String` or `PathBuf` etc) you can ofc always just clone.

There's also `.er_with_tree()` if you need something from the tree:

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

The thing that this has over map_err is that it makes you not accidentally nuke the tree. It's the operation that modifies the tree (not for giving it up at the end/consuming), so the compiler can help you.

## Reusing context / closures

You can make little helpers if you just want to have the same dynamic context (that changes at runtime), but you want a different message (or other things that can be different).

```rust
#[derive(Er)]
pub struct SyncErr {
    pub path: PathBuf,
    pub host: String,
    pub port: u16,
    pub msg: &'static str,
}

pub fn sync(path: &Path, host: &str, port: u16) -> ErResult<(), SyncErr> {
    let e = |msg| (path, host, port, msg);

    let data = read_file(path).er(e("read file"))?;
    if data.is_empty() {
        er_bail!(e("empty file"));
    }

    connect(host, port).er(e("connect"))?;
    upload(&data).er(e("upload"))?;

    Ok(())
}
```

When they're all `Copy` (references like `&Path`, `&str`, `&String`, `&PathBuf`, numbers, etc.), it's a tuple with zero allocations on the ok path, so you don't need another closure at `.er(e("..."))?` (you can also do it with `er_bail!` and `er_all!`).

If a field needs a lookup like `user.name()` or you're building an enum variant, you should return a closure with `e`.

```rust
let e = |msg| move |_| (path, user.name(), msg);
// or for an enum:
let e = |msg| move || SyncErr::failed(path, host, msg);

read_file(path).er(e("read file"))?;
```

## Print report

If you want to keep the tree, you can call `.er_report()` (if you want to pass it on at a public boundary, check out the `er_split()` part).

```rust
if let Err(error) = read_port("fakenumber") {
    eprintln!("{}", error.er_report());
}

// Yolo unwrap or `.er_expect("bad port")`
let port = read_port("85").er_unwrap();
```

If you want to give up the error, this also works.

```rust
let result: Result<u16, String> = read_port("85").er_report();
```

For loggers with each line separately you can use `error.for_each_line(|line| log_line(line))?`.

Your `pub fn main()` can return `ErMain`. You use `.er(())?` like other unit Er errors (prints report).

```rust
pub fn main() -> ErMain {
    read_port("85").er(())?;
    Ok(())
}
```

If your program picks its own exit status, return `ErMain<AppErr, ExitCode>` and finish with `Ok(code)`.

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

## Helpers / Constructors

Use the return type to your advantage aka just pass fields to `.er()` in order and it handles the conversions if something fails:

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

You don't need `ReadConfigErr::new(...)` (which is also generated), `path.to_path_buf()` or `String::from(...)` at the call, and no turbofish either since the return type already names `ReadConfigErr` (you only need `.er::<SubErr>(..)` when nothing names it yet, like inside `er_all!` or mid chain, but you don't need chains). The same field helpers work with `.er_with()` and `.er_val()`.

Copy stuff like `&Path` or `&str` goes straight in and only converts if the call fails. For owned values or lookups like `user.name()`, you can use a closure `.er(|_| (path, user.name()))` so the ok path skips them too.

You still get `new(...)` for constructing a value directly, and `.er(|| MyTypeErr { a, b })` works for a finished error.

You can pass values directly without `.as_ref()` or `.as_str()`, and `Option<T>` fields take `T` without `Some(...)` (for `Option<String>` that means a `String`, so from a `&str` use `.er(|_| msg.to_owned())`).

### Default fields

If you put `#[er(default)]` (or `#[er(default = expr)]`) on fields at the end of a struct, you can leave them off in `.er(..)` / `er_bail!(..)` / `.er_with(..)`:

```rust
#[derive(Er)]
pub struct ReadErr {
    pub path: PathBuf,
    #[er(default)]
    pub attempts: u8,
    #[er(default = 85)]
    pub size: u64,
}

fs::read(path).er(path)?; // attempts is 0, size is 85
fs::read(path).er((path, 3))?; // size is 85
fs::read(path).er((path, 3, len))?;
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

It's just Arc/Rc depending on the platform, don't wrap values in `ErShared::new(...)` before the call, just pass `path` (or `&path` if you only have a reference).

## Sharing / reusing errors

If you need to reuse/share the result/error you can freeze with `.er_share()` (`er_find()` will still work):

```rust
pub struct Device {
    pub opened: Result<Speaker, ErSharedTree<OpenErr>>,
}

impl Device {
    pub fn open(&mut self) {
        self.opened = open_speaker().er_share();
    }

    pub fn capture(&self) -> ErResult<&Speaker, CaptureErr> {
        self.opened.as_ref().er_with(|err| err.device.clone())
    }
}
```

`ErSharedTree` is `Clone` and read only, and like `ErTree` it doesn't implement `Debug` or `Display` (`failure.top` and `.er_report()` also work). 

If you need `Debug` or you're done building the tree, use `.er_split()` for `(MyError, String)` instead (and derive `Clone` on your error if you also need to clone it).

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
    let host = read_file(path).er(top_err)?; // first use

    er_all!(top_err, [ // second use
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
        Err(second_error) => Err(first_error.er_add(second_error)),
    }
}
```

When you want to add multiple errors to a previous one, `er_add!` lets them have different types (it drops the oks from results like `er_all!`):

```rust
error = er_add!(error, [first_error, second_error]);
```

When you're building a summary, add each detail directly. Plain errors have no location of their own, including when they come from a failed `Result`. A tree from `.er()` or `er_new!()` keeps the location where you made it.

```rust
report = report.er_add(failed.er_save("report", worker_report));
```

Starting from a plain error gives you an `ErBranch`. You can keep adding children and saved reports, and `er_bail!` turns it into a tree with the bail location on top:

```rust
let error = ImportErr::rejected(path).er_add(rollback_import());

er_bail!(error);
```

## Find original error

`.er_find()` also checks `source()`, `.er_find_all()` gets every match and `.er_contains()` checks if a type is in there.

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

When you (as a library) want to give your consumer an error, don't give them an `ErTree`, give them a boring normal error (with `#[derive(Er)]`) and hand the report out next to it with `.er_split()`:

```rust
#[derive(Er)]
pub enum PortError {
    Missing,
    // You can just use the normal formatting, just showing er format here if you prefer that style for public / boundary errors.
    #[er(format = "Not a port: {input}")]
    InvalidPort { input: String },
}

pub fn read_port(input: &str) -> Result<u16, (PortError, String)> {
    input.parse().er(|| PortError::invalid_port(input)).er_split()
}

// Consumer does not need/depend on er by you exposing your type
if let Err((PortError::InvalidPort { input }, report)) = read_port("nope") {
    eprintln!("Fix the port in your config: {input}\n{report}");
}
```

Once a function returns `Result<_, (MyError, String)>`, `?` and `er_bail!` split for you automatically, so you don't need to manually call `.er_split()`:

```rust
pub fn read_config(path: &Path) -> Result<Config, (ConfigError, String)> {
    let text = fs::read_to_string(path).er(|| ConfigError::unreadable(path))?;
    if text.trim().is_empty() {
        er_bail!(ConfigError::empty(path));
    }

    Ok(parse(&text))
}
```

For an API or service where you want to log the full report yourself and only hand back the public error you can use `.er_split_with(...)`:

```rust
async fn create_user(Json(user): Json<NewUser>) -> Result<Json<User>, UserError> {
    let user = insert_user(user).await.er_split_with(|(error, report)| {
        error!("{report}");
        error
    })?;

    Ok(Json(user))
}
```

It's important to note that the er apis like .er() automatically include the (Error, String) and use it as a saved attachment on the tree, which means that if you follow the pattern you don't have to make boilerplate yourself.

## Reports from somewhere else

Say a library gives you `(StartError, String)` (or `(StartError, Option<String>)`). Add what you were doing and Er saves their report under it (if it's `None` or blank, you still get `StartError` and its causes).

```rust
let started = audio::start(setup).await.er(|| AudioErr::stopped(name))?;
```

```text
AudioErr::Stopped { name: "speaker" } @ src/audio.rs:88:31
`- audio startup failed
   `- audio::StartError [saved]
        audio startup failed for speaker
        OpenErr { path: "app.db" } @ src/device.rs:39:10
        `- disk full
```

If their report sits in a field (or a server's response body), you can manually save with `.er_save()`:

```rust
client.send(request).er_save("server", |err| &err.body).er(id)?;
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

You can put the value on the error this function returns. If you need conversion logic, you can give the field its own data type and let the tuple helpers call `.into()` for you, instead of turning the value into another error and adding a layer above it.

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

## Bail / return

For returning you can use `er_bail!(err)` (don't use `return Err(ErTree::from(err))`):

```rust
if path.is_empty() {
    er_bail!((path, "no file given"));
}

// With an existing error as the cause
Err(error) => er_bail!(LoadErr::open(&path), error),
```

Inside a closure or an `async` block, Rust can know which error you have and still need you to say the block returns an `ErResult`. End it with `ErOk(value)`.

```rust
let body = async {
    let data = match fetch(url).await {
        Ok(data) => data,
        Err(error) => er_bail!(ReadErr::failed(url), error),
    };

    ErOk(data.len())
};
```

If the error type also needs a name, write `ErOk::<ReadErr, _>(value)`. When a block only ever fails and the caller takes a future, you can name the return type on an `async fn` and pass `offline()` to the caller.

```rust
async fn offline() -> ErResult<(), ReadErr> {
    er_bail!(ReadErr::offline());
}
```

## Snapshots

Saves msgs and the tree structure for christmas or other special occasions (as strings, not the error types).

```rust
if let Err(error) = read_port("fakenumber") {
    let snapshot = error.er_snapshot();
    drop(error);

    println!("{}", snapshot.er_report());

    for entry in &snapshot.entries {
        println!("{}", entry.message);
        for saved in &entry.saved {
            println!("  {}", saved.title);
        }
    }
}
```

A report you got as an `(E, String)` pair is one text attachment. A GUI can put that whole string under its title. If you saved an `ErSnapshot`, its entries are still there to draw. When you keep saving the previous snapshot on every retry, the newest eight saved snapshots keep their rows and the older history stays as text.

You can use `.top()` or `.er_report()` and on a Result `.er_snapshot()` saves Err and leaves Ok alone.

```rust
let result: Result<u16, ErSnapshot> = read_port("fakenumber").er_snapshot();
```

You can also attach an older snapshot to a new error with `.er_save()` if an earlier failure helps explain what happened (like a sync that failed earlier and left you offline):

```rust
let last_sync = sync(&db).await.er_snapshot().err();

reconnect(&server).er(|_| ReconnectErr.er_save("last sync", &last_sync))?;
```

```text
ReconnectErr @ src/net.rs:21:17
|- connection refused
`- last sync [saved]
     SyncErr @ src/sync.rs:10:24
     `- database is locked
```

## Tests

If your test returns `ErTest` you can just use `?`. Ordinary errors and Er trees get `TestError` on top with the location of the `?` and keep the original errors below it:

```rust,ignore
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

Test helper functions should return normal error types (not `ErTest`), since `?` passes an `ErTest` error through without adding context. For Options, `Result<T, ()>` and `(Error, String)` pairs you can use `.er_test()?`.

When a test expects a failure you can use `er_assert_err!`. It fails on Ok or the wrong error and prints the whole report so you see what you got instead:

```rust
#[test]
pub fn rejects_words() -> ErTest {
    er_assert_err!(read_port("nope"), ReadPortErr);
    Ok(())
}
```

If you want to inspect the error yourself you can do `let error = er_assert_err!(load(path));` since it gets returned.

## Other traits

If you need to implement another crate's trait (like Axum's `IntoResponse`) on the whole tree, you can use `#[er(wrap)]` so you own the type:

```rust
#[derive(Er)]
#[er(wrap(name = HandlerError))] // Defaults to HandlerErrWrap without name
pub struct HandlerErr;

pub fn handler(input: &str) -> Result<u16, HandlerError> {
    let port = input.parse().er(())?;
    Ok(port)
}
```

If you use `?` the tree will be in `HandlerError` and callers can keep using `.er(())` on it like always. See [macros.md#wrap](https://github.com/Viterkim/er/blob/main/er/docs/macros.md#wrap) for the full Axum example and `std_error` (which you almost never need).

## Foreign callbacks

For cases like where SQLite chooses the error type for `FromSql` our callback can't return our tree (it gives us a box for an ordinary error) so we put the invalid value on the error and use a manual src_location field:

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

## Stack traces

With the `stack_traces` feature which needs `std`, you can add `.er_save("stack trace", Backtrace::force_capture)` where you want to capture the stack.

```rust
use std::backtrace::Backtrace;

pub fn read_port(input: &str) -> ErResult<u16, ReadPortErr> {
    input.parse().er(|_| ReadPortErr.er_save("stack trace", Backtrace::force_capture))
}

if let Err(error) = read_port("fakenumber") {
    eprintln!("{}", error.er_report());
}
```

They follow the tree as you add context or aggregate it. They print under the error you saved them on. You can always call `.er_save()` again and there's no env variables to turn on.

## Formatting your types

The default output shows your type and its fields. You can give Display a different message while Debug keeps the full struct view:

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

See [macros.md](https://github.com/Viterkim/er/blob/main/er/docs/macros.md) for `format`, `debug`, `display_alt`, `skip`, `censor` and custom formatting functions.

## Bigger example

For a full example combining foreign errors, custom rules, `.er_with()`, matching on `.top` and a public boundary (compared against other error crates), see [the tricky comparison](https://github.com/Viterkim/er/blob/main/er/docs/tricky-error-comparison.md#er-0100-1).
