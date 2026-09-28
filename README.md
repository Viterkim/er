# Er, errors with less r

Easy and convenient typed error handling at every step (for applications and libraries).

Design your error types around what your caller cares about, not what combination of errors you got.

```toml
[dependencies]
er = "0.5"
```

[Full examples / usage patterns](er/docs/examples.md)

## Simple example

Use `.er()` on any result/error/option/tree, even on different types. A tree/report is built and the original typed error is below (file names and line numbers are also added).

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
    read_to_string(path).er(|_| path)
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
let port = input.parse::<u16>().er(|_| input)?;

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
    // Closures only run on failure
    let e = |_| (port, enabled);

    read_config_file(path).er(e)?;
    port.parse::<u16>().er(e)?;
    enabled.parse::<bool>().er(e)?;

    Ok(())
}

// variant 2: Aggregate/collect errors, runs all and errors if any failed
pub fn check_config_collect(path: &Path, port: &str, enabled: &str) -> ErResult<(), ConfigErr> {
    let e = |_| (port, enabled);

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

Please check out the full [examples / patterns docs](er/docs/examples.md) which show most patterns you can use with 'er' (but below are some important notes).

### Making errors

Most functions that deal with errors, should have their own error type.

```rust
// Just put your error type directly above your function.
#[derive(Er)]
pub struct MyFuncErr;

pub fn my_func() -> ErResult<(), MyFuncErr> {
  something_else().er(())?;
  // more real code
}
```

This forces you to re add context with `.er()` (otherwise you can just '?' the same error up all the way).

Your errors should not be giant pyramids, they should be local to the things you are doing, related to what your consumer cares about and not which errors you got (the original errors are always stored automatically in the tree below).

Think about what your caller wants to 'match on', or what is relevant for the flow/logic of your code.

You can carry data up instead of runtime searching(`.er_find()`) by using `.er_with(|e|)` to inspect the previous error. You can also use `ErShared<DataType>` on your fields in your error types to avoid cloning the value.

### Helpers / constructors

Structs: Use `.er(())` for empty structs, `.er(|_| path)` for 1 field structs, and `.er(|_| (port, enabled))` for 2 or more fields.

Enums: Need a variant specified like `.er(|| ModeErr::variant_name(arg1))`.

Just use the helpers then the type is inferred, but you can also manually write out `.er(|| MyTypeErr { a, b })`. The helpers take care of stuff like not having to call `.into()` or type the error name, which is why you should just do `.er(())?` instead of `.er(|| MyGoodError)?`.

### Don't use .map_err()

Do NOT use `map_err`. It's easy to accidentally nuke the tree/report. It should only ever be used at the final boundary when you are 'done' with your tree/report.

### Picking between `Report` and `Top` (or unwrapping)

To avoid Debug/Display implicitly meaning either a report or the top error, you need to specify which you want with `.er_report()` or `.er_top()`. But... if you really miss unwrapping there's `.unwrap_report()` or `.expect_report("message")`.

For main you can write: `pub fn main() -> Result<(), ErReport<AppErr>>`, and for tests you can use `pub fn test_name() -> ErTest` and `?` errors without `.er()`.

### Give your public consumer a non 'er' error

When you (as a library) want to give your consumer an error, don't give them an `ErReport` or an `ErTree`, give them a boring normal error (with `#[derive(Er)]`).

Decide if you want to `error!("{report}")` before mapping your type (saying goodbye to it with `.map_err()`). But look at the [public error example](er/docs/examples.md#public-error)

## Philosophy

The distinction should not be app/lib error handling, it should be public consumer/internal consumer based, and `er` does both.

Worse errors/types lead to worse logic/flow because error states get grouped into impossible cases. If a function cannot return a 'serde error', why does it return a type that says it can? Ergonomics are important to make this easier.

The original inner error should not dictate your error type design or be given to your final consumer directly.

It's not the user's fault when mistakes happen. There should be convenience and helpers to avoid footguns if possible (`.er_with(|e|)`, `.er_find::<SomeErr>()`, `ErTree` not having `Debug` / `Display`).

## Docs

### Repo links (from `./er/docs/`)

[Examples / Patterns](er/docs/examples.md)

[Changelog](er/docs/changelog.md)

[Features](er/docs/features.md)

[Macros](er/docs/macros.md)

[Github Repo](https://github.com/Viterkim/er)

### External links

[Docs.rs](https://docs.rs/er/latest/er/)

[Crates.io for the lib](https://crates.io/crates/er)

[Crates.io for the macros](https://crates.io/crates/er-macros/)
