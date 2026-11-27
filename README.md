# Er, errors with less r

Easy and convenient typed error handling (for applications and libraries).

Design typed errors around what your caller cares about, not what errors you get.

```toml
[dependencies]
er = "0.10"
```

[Full examples / usage patterns](er/docs/examples.md)

## Simple example

Use `.er()` on any result/error/option/tree to make an error tree/report (even on different types).

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
#[derive(Er)]
pub struct ReadConfigFileErr;

pub fn read_config_file(path: &Path) -> ErResult<String, ReadConfigFileErr> {
    // Still gets source location
    read_file(path).er(())
}

#[derive(Er)]
pub struct ConfigErr {
    pub port: String,
    pub enabled: String,
}

// Exit on the first error
pub fn check_config_exit_early(path: &Path, port: &str, enabled: &str) -> ErResult<(), ConfigErr> {
    // Nothing gets built unless it fails
    let e = (port, enabled);

    read_config_file(path).er(e)?;
    port.parse::<u16>().er(e)?;
    enabled.parse::<bool>().er(e)?;

    Ok(())
}

// Aggregate/collect errors
pub fn check_config_collect(path: &Path, port: &str, enabled: &str) -> ErResult<(), ConfigErr> {
    let e = (port, enabled);

    // Different error types are fine
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
|- invalid digit found in string
`- provided string was not `true` or `false`
```

## Using 'er'

For all patterns/how to use 'er' you (or your agent) should read [examples / patterns docs](er/docs/examples.md), with common mistakes in [good and bad](er/docs/good-and-bad.md).

### Helpers / constructors

Structs: `.er(())` for empty, `.er(path)` for 1 field, and `.er((port, enabled))` for 2+ fields.

Enums: specify variant like: `.er(|| ModeErr::variant_name(arg1))`.

Copy + Debug values/references (like `&Path`) you can just write without a closure because they only get converted when the call fails.

For owned values, or a lookup like `user.name()` you should use a closure so they only run on failure: `.er(|_| (path, user.name()))`.

Just use the helpers then the type is inferred, but you can also manually write out `.er(|| MyTypeErr { a, b })`. The helpers take care of stuff like not having to call `.into()` or type the error name, which is why you should just do `.er(())?` instead of `.er(|| MyGoodError)?`.

### Making errors

Most functions that deal with errors should have their own error type, directly above the function. That forces you to add what this function was doing with `.er()`, otherwise you can just `?` the same error all the way up.

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
    // Include messages in the type itself, don't use `#[er(format)]` for that
    pub msg: &'static str, // Bonus: even if it was an Option<>, Some(v)/.into() wouldn't be needed
}

pub fn other_func(path: &str) -> ErResult<(), MyOtherErr> {
    // Usually pass values directly, no .as_ref()/.as_str(), and Option<T> takes T without Some().
    if path.is_empty() {
        er_bail!((path, "no file given"));
    }
    my_func().er((path, "some cool msg"))
}
```

Your errors shouldn't be huge pyramids. Give your caller data to control logic/flow (the lower errors are there for printing below automatically, and your data/flow is what you decide). 

Rust has a type system you can use so put your message separately as a field and have the other data alongside instead of abusing 'String' and `#[er(format)]` (er will format it automatically so you can see it all).

Carry data up with `.er_with(|err|)` instead of runtime searching (`.er_find()`), and use `ErShared<T>` on fields if you want to share data without cloning.

### Don't use .map_err()

Do NOT use `map_err` with Er. It's easy to accidentally nuke the tree/report, and there's `.er_split()` for the final boundary (there's also `.er_report()` for just the String).

### Picking between the report and the top (or unwrapping)

To avoid Debug/Display implicitly meaning either a report or the top error, you need to specify which you want with `.er_report()` or `error.top`.

### Give your public consumer a non 'er' error

When you (as a library) want to give your consumer an error, don't give them an `ErTree`(it's a felony), give them a boring normal error (with `#[derive(Er)]`).

You can use `.er_split()` to hand them `(MyErr, String)`. The error to match on and the report to print (they don't need Er, or the same version as you). 

```rust
// Library where they might care: give em' the error and the report
pub fn open(path: &Path) -> Result<File, (OpenError, String)> {
    open_file(path).er_split()
}
```

Or for cases like a service/API where you want to log the report, and only return the error itself you can use `.er_split_with(...)`:

```rust
// Service / API: log the full report yourself, only send back the public error
pub fn open_public(path: &Path) -> Result<File, OpenError> {
    open_file(path).er_split_with(|(error, report)| {
        error!("{report}");
        error
    })
}
```

Even as a library, handing your consumer that string report means way better bug reports from users and a much easier time debugging than some vague friendly message (and thiserror people can impl from (YourError, String)).

## Philosophy

The distinction should not be app/lib error handling, it should be consumer/boundary based.

Worse errors/types lead to worse logic/flow.

The inner error should not dictate your error type design.

It's not the user's fault when mistakes happen. There should be convenience and helpers to avoid mistakes (`.er_with(|err|)`, `ErTree` not having `Debug` / `Display`, `er_split()` etc).

## Unsync / Single threaded platform

If you've got a platform that uses Rc/RefCell (single threaded embassy etc) you can use `er-unsync` (name it `er` so the derives can find it).

```toml
er = { package = "er-unsync", version = "0.10" }
```

## Docs

### Repo links (from `./er/docs/`)

[Examples / Patterns](er/docs/examples.md)

[Good and bad](er/docs/good-and-bad.md)

[Design decisions that might seem weird](er/docs/special-design.md)

[Changelog](er/docs/changelog.md)

[Features](er/docs/features.md)

[Macros](er/docs/macros.md)

[GitHub Repo](https://github.com/Viterkim/er)

### External links

[Docs.rs](https://docs.rs/er/0.10.0/er/)

[Crates.io for the lib](https://crates.io/crates/er)

[Crates.io for the macros](https://crates.io/crates/er-macros/)

[Crates.io for the unsync lib](https://crates.io/crates/er-unsync)

[Libs.rs](https://lib.rs/crates/er)
