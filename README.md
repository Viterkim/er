# Er, errors with less r

Easy and convenient typed error handling at every step (for applications and libraries).

Design your error types around what your caller cares about, not what combination of errors you got.

```toml
[dependencies]
er = "0.9"
```

[Full examples / usage patterns](er/docs/examples.md)

## Simple example

Use `.er()` on results, errors, options and trees, even on different types. Here a failed read puts `FileErr` on top and keeps the original I/O error underneath, with the file and line where you added context.

```rust
use er::*;
use std::{fs::read_to_string, path::{Path, PathBuf}};

// 'Er' implements `Error`, `Display`, `Debug` and creates helpers.
#[derive(Er)]
pub struct FileErr {
    pub path: PathBuf,
}

pub fn read_file(path: &Path) -> ErResult<String, FileErr> {
    // Extra context with 'path' + automatic source location
    read_to_string(path).er(path)
}
```

Printed with `.er_report()`

```text
FileErr { path: "/file/path/missing.txt" } @ src/main.rs:11:26
`- No such file or directory (os error 2)
```

## Comparison to anyhow/thiserror/error-stack

### With manual extra context

```rust
// er
let port = input.parse::<u16>().er(input)?;

// thiserror (typed but no tree/report)
let port = input.parse::<u16>().map_err(|source| PortErr { input: input.into(), source })?;

// anyhow
let port = input.parse::<u16>().with_context(|| format!("invalid port {input:?}"))?;

// error-stack (tree/report)
let port = input.parse::<u16>().change_context_lazy(|| PortErr { input: input.into() })?;
```

```rust
// er
#[derive(Er)]
pub struct PortErr {
    pub input: String,
}

// thiserror
#[derive(Debug, thiserror::Error)]
#[error("invalid port {input:?}: {source}")]
pub struct PortErr {
    pub input: String,
    pub source: std::num::ParseIntError,
}

// error-stack (using thiserror for the type)
#[derive(Debug, thiserror::Error)]
#[error("invalid port {input:?}")]
pub struct PortErr {
    pub input: String,
}
```

```text
// er
PortErr { input: "aint_even_a_number_cmon_man" } @ examples/er_context.rs:8:35
`- invalid digit found in string

// thiserror
invalid port "aint_even_a_number_cmon_man": invalid digit found in string

// anyhow
invalid port "aint_even_a_number_cmon_man"
Caused by:
    invalid digit found in string

// error-stack
invalid port "aint_even_a_number_cmon_man"
├╴at examples/error_stack_context.rs:10:37
│
╰─▶ invalid digit found in string
    ╰╴at examples/error_stack_context.rs:10:37
```

### Without manual extra context

```rust
// er
#[derive(Er)]
pub struct PortErr;

// thiserror
#[derive(Debug, thiserror::Error)]
pub enum PortErr {
    #[error("parse failed: {0}")]
    Parse(#[from] std::num::ParseIntError),
}

// error-stack (using thiserror for the type)
#[derive(Debug, thiserror::Error)]
#[error("PortErr")]
pub struct PortErr;
```

```text
// er
PortErr @ examples/er_no_context.rs:7:35
`- invalid digit found in string

// thiserror
parse failed: invalid digit found in string

// anyhow
invalid digit found in string

// error-stack
PortErr
├╴at examples/error_stack_lazy.rs:9:37
│
╰─▶ invalid digit found in string
    ╰╴at examples/error_stack_lazy.rs:9:37
```

```rust
// er
let port = input.parse::<u16>().er(())?;

// thiserror / anyhow
let port = input.parse::<u16>()?;

// error-stack
let port = input.parse::<u16>().change_context(PortErr)?;
```

## Detailed comparisons

If you want bigger / more detailed comparisons for `thiserror, anyhow, snafu, error-stack, rootcause, exn, eros, problemo` or you are thinking "why not one of those?"

[Simple comparison: basic usage, context and output](er/docs/simple-error-comparison.md)

[Tricky comparison: multiple errors, using their data and public boundaries](er/docs/tricky-error-comparison.md)

## Putting it all together

```rust
// -- First part --
#[derive(Er)]
pub struct ReadConfigFileErr;

pub fn read_config_file(path: &Path) -> ErResult<String, ReadConfigFileErr> {
    // Still gets source location
    read_file(path).er(())
}

// -- Second part somewhere else --
#[derive(Er)]
pub struct ConfigErr {
    pub port: String,
    pub enabled: String,
}

// variant 1: Exit on the first error
pub fn check_config_exit_early(path: &Path, port: &str, enabled: &str) -> ErResult<(), ConfigErr> {
    // Nothing gets built unless it fails
    let e = (port, enabled);

    read_config_file(path).er(e)?;
    port.parse::<u16>().er(e)?;
    enabled.parse::<bool>().er(e)?;

    Ok(())
}

// variant 2: Aggregate/collect errors, runs all and errors if any failed
pub fn check_config_collect(path: &Path, port: &str, enabled: &str) -> ErResult<(), ConfigErr> {
    let e = (port, enabled);

    // Different error types are fine, adds the sub errors to the parent if anything fails
    er_all!(e, [
        read_config_file(path),
        port.parse::<u16>(),
        enabled.parse::<bool>(),
    ])
}
```

```text
ConfigErr { port: "nope", enabled: "nah" } @ src/main.rs:46:5
|- ReadConfigFileErr @ src/main.rs:19:21
|  `- FileErr { path: "/file/path/missing.txt" } @ src/main.rs:11:26
|     `- No such file or directory (os error 2)
|- invalid digit found in string @ src/main.rs:48:9
`- provided string was not `true` or `false` @ src/main.rs:49:9
```

## Using 'er'

The [examples](er/docs/examples.md) go from wrapping one error to collecting a whole batch. The mistakes we keep seeing are in [good and bad](er/docs/good-and-bad.md).

### Helpers / constructors

Give `.er()` the fields in order. Use `.er(())` for an empty struct, `.er(path)` for one field or `.er((port, enabled))` for two. For an enum, choose the variant with `.er(|| ModeErr::unknown(mode))`.

Pass values like a path reference straight in when they implement `Copy + Debug`. Er converts the reference to your field type when the call fails. Pass owned fields in a closure. Put a lookup like `user.name()` there too, `.er(|_| (path, user.name()))`, so it only runs when the call fails.

Your function's return type already says which struct to build, so `.er(())?` is enough. You can still write out `.er(|| MyTypeErr { a, b })` when you want the named fields.

### Making errors

Most functions that deal with errors should have their own error type, directly above the function. That forces you to add what this function was doing with `.er()`, otherwise you can just `?` the same error all the way up.

```rust
#[derive(Er)]
pub struct MyFuncErr;

pub fn my_func() -> ErResult<(), MyFuncErr> {
    something_else().er(())?;
    // more real code
}

#[derive(Er)]
pub struct MyOtherErr {
    pub path: PathBuf,
    pub msg: &'static str,
}

pub fn other_func(path: &str) -> ErResult<(), MyOtherErr> {
    if path.is_empty() {
        er_bail!((path, "no file given"));
    }
    my_func().er((path, "some cool msg"))
}
```

Your errors should not be giant pyramids. Give your caller the cases and data they can actually use, the original errors are already in the tree below.

The derive already prints the fields. For messages you want in the report, put them on the type like any other data.

If your caller can retry a busy device, give them a `Busy` variant to match on. Pick that variant one layer up with `.er_with(|err| ..)`. Making them dig through the tree with `.er_find()` means their retry quietly stops working the day you change the error underneath. Use `ErShared<T>` when several layers need to keep the same data.

### Don't use .map_err()

Do NOT use `map_err` with Er. It's easy to accidentally nuke the tree/report. At your library boundary, `.er_split()` hands your caller both the typed error and its report.

### Printing

Match on the Result. In the `Err(error)` arm, `error.top` is yours to match on and `error.er_report()` gives you the whole report to print. Calling `.er_report()` on the `ErResult` gives you `Result<T, String>`, so do that when text is all you came for. `.er_unwrap_report()` and `.er_expect_report("message")` panic with the report.

Return `ErMain` from main or `ErTest` from a test and a plain `?` prints the report on failure. Use `.er_test()?` on a library's `(Error, String)` result to keep its error and report.

### Give your public consumer a non 'er' error

Say your library opens a file. Your caller can do something about `OpenError::Denied`. When they send you a bug report you probably want the path, the OS error and where it happened too.

```rust
pub fn open(path: &Path) -> Result<File, (OpenError, String)> {
    open_file(path).er_split()
}
```

They can match on your enum and print the String whatever Er version either of you uses, even if they have no Er dependency. With Er 0.9 on their side, their usual `.er()` keeps your error and saves your report underneath. Keep the fields on that public error ordinary too. A public `ErShared<Device>` field still puts Er in their API.

An HTTP service can use `.er_split_with(...)` to log the report where it runs and send just the public error to the client. [Public error examples](er/docs/examples.md#public-error).

## Philosophy

Use the tree anywhere the next piece of code is yours, in a library or in an app. At the public API you hand over the cases and data somebody else can use.

Worse errors/types lead to worse logic/flow because error states get grouped into impossible cases. If a function cannot return a 'serde error', why does it return a type that says it can? Ergonomics are important to make this easier.

The original inner error should not dictate your error type design or be given to your final consumer directly.

A tree has no `Debug` or `Display` on purpose. It'd be way too easy to turn a typed error into text halfway up the stack.

[A few odd things about Er, from the public tuple to the extra `.er()` before `?`](er/docs/special-design.md)

## Unsync / Single threaded platform

Use `er-unsync` to keep `Rc` in sub errors or to share trees on targets without pointer atomics. Keep the dependency name `er` so the derives can find it.

```toml
er = { package = "er-unsync", version = "0.9" }
```

## Docs

### Repo links (from `./er/docs/`)

[Examples / Patterns](er/docs/examples.md)

[Good and bad](er/docs/good-and-bad.md)

[Changelog](er/docs/changelog.md)

[Features](er/docs/features.md)

[Macros](er/docs/macros.md)

[GitHub Repo](https://github.com/Viterkim/er)

### External links

[Docs.rs](https://docs.rs/er/0.9.0/er/)

[Crates.io for the lib](https://crates.io/crates/er)

[Crates.io for the macros](https://crates.io/crates/er-macros/)

[Crates.io for the unsync lib](https://crates.io/crates/er-unsync)

[Libs.rs](https://lib.rs/crates/er)
