# Er, errors with less r

Easy and convenient typed error handling at every step (for applications and libraries).

Design your error types around what your caller cares about, not what combination of errors you got.

```toml
[dependencies]
er = "0.2"
```

[Full examples / usage patterns](er/docs/examples.md)

## Simple example

Use `.er()` on any result/error/option/tree, even on different types.

`er` builds a tree/report and keeps the original typed error below. It also adds compile time file names/line numbers.

```rust
use er::*;
use std::{fs::read_to_string, path::{Path, PathBuf}};

#[derive(Er)]
pub struct FileErr {
    pub path: PathBuf,
}

pub fn read_file(path: &Path) -> Er<String, FileErr> {
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

### With context

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

### Without context

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

Overview of a [minimal error example compared (basic usage, context, output)](er/docs/simple-error-comparison.md)

There's also a [huge tricky error comparison (foreign errors, own errors, the original error, string context, typed context, using / consuming, public boundary)](er/docs/tricky-error-comparison.md)

## Putting it all together

Also check out the [full list of examples/patterns for 'er'](er/docs/examples.md)

```rust
// -- First part --
#[derive(Er)]
pub struct NoContextErr;

pub fn no_context_example(path: &Path) -> Er<String, NoContextErr> {
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
pub fn check_config_exit_early(path: &Path, port: &str, enabled: &str) -> Er<(), ConfigErr> {
    // Closures only run on failure
    let e = |_| (port, enabled);

    no_context_example(path).er(e)?;
    port.parse::<u16>().er(e)?;
    enabled.parse::<bool>().er(e)?;

    Ok(())
}

// variant 2: Aggregate/collect errors, runs all and errors if any failed
pub fn check_config_collect(path: &Path, port: &str, enabled: &str) -> Er<(), ConfigErr> {
    let e = |_| (port, enabled);

    // Different error types are fine, adds the sub errors to the parent if anything fails
    er_all!(e, [
        no_context_example(path),
        port.parse::<u16>(),
        enabled.parse::<bool>(),
    ])
}
```

```text
ConfigErr { port: "nope", enabled: "nah" } @ src/main.rs:46:5
|- NoContextErr @ src/main.rs:19:21
|  `- FileErr { path: "/file/path/missing.txt" } @ src/main.rs:11:26
|     `- No such file or directory (os error 2)
|- invalid digit found in string @ src/main.rs:49:9
`- provided string was not `true` or `false` @ src/main.rs:50:9
```

## Motivations

### Convenience

`#[derive(Er)]` generates helpers to avoid stuff like: `.change_context_lazy(|| ConfigErr { port: port.to_owned(), enabled: enabled.to_owned() })?`.

`er_all!()` can collect different types (doesn't quit out early).

`.er_with(|e|)` for interacting with the typed error below without accidentally destroying the tree (easy to accidentally do with `.map_err()`).

`.er_find::<SomeErr>()` for the first match, and `.er_find_all::<SomeErr>()` for finding the original errors and suberrors below (also checks `.source()`).

`wrap` for implementing foreign traits.

`use er::*;` should be usable without shadowing normal types.

Easy creation of public errors for consumers who don't have/want `er`.

### Philosophy

The distinction should not be app/lib error handling, it should be public consumer/internal consumer based, and `er` does both.

Worse errors/types lead to worse logic/flow because error states get grouped into impossible cases. If a function cannot return a 'serde error', why does it return a type that says it can? Ergonomics are important to make this easier.

The original inner error should not dictate your error type design or be given to your final consumer directly.

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
