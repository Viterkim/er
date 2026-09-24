# Simple Error Comparison ™

Same bad port for every library. First the easy version, then one with the input added so we can actually see what went wrong.

Links on this page: [thiserror](#thiserror-2020), [error-stack](#error-stack-080), [Er](#er-020), [Anyhow](#anyhow-10104), [SNAFU](#snafu-092), [rootcause](#rootcause-0130),
[exn](#exn-031), [Eros](#eros-080-rc1), [Problemo](#problemo-0013), [Nightly std::error::Report](#nightly-stderrorreport), [Others](#others)

For the messy version with several errors and a public boundary, see the [tricky comparison](tricky-error-comparison.md).

## The quest /task we're about to do

Someone entered `aint_even_a_number_cmon_man` as input and we get this:

```text
invalid digit found in string
```

So let's call `read_port("aint_even_a_number_cmon_man")` and try adding the input with each library.

Done on Rust 1.98.1

## thiserror (2.0.20)

Helper macro for doing `From<E>`, `Error` and `Display`. [Docs](https://docs.rs/thiserror/2.0.20/thiserror/)

### Lazy

An enum of the things that can fail, then just `?`.

```rust
use std::num::ParseIntError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PortErr {
    #[error("parse failed: {0}")]
    Parse(#[from] ParseIntError),
}

pub fn read_port(input: &str) -> Result<u16, PortErr> {
    let port = input.parse()?;
    Ok(port)
}

println!("{error}");
println!("{error:?}");
```

```text
parse failed: invalid digit found in string
Parse(ParseIntError { kind: InvalidDigit })
```

First line is Display, second is Debug. Neither knows we passed `aint_even_a_number_cmon_man`.

### With context

Now we want the input.

```rust
#[derive(Debug, Error)]
pub enum PortErr {
    #[error("invalid port {input:?}: {source}")]
    Parse {
        input: String,
        source: ParseIntError,
    },
}

pub fn read_port(input: &str) -> Result<u16, PortErr> {
    let port = input.parse().map_err(|source| PortErr::Parse {
        input: input.to_owned(),
        source,
    })?;
    Ok(port)
}

println!("{error}");
println!("{error:?}");
```

```text
invalid port "aint_even_a_number_cmon_man": invalid digit found in string
Parse { input: "aint_even_a_number_cmon_man", source: ParseIntError { kind: InvalidDigit } }
```

There goes `#[from]`, it gets the parser error but not our local `input`. So we write the `map_err` and source field ourselves. The caller does get the `Parse` case and can read its source directly.

### Unique case

The upside of declaring the source, the caller can just take it out. No search or downcast. Same goes for SNAFU's typed errors below.

```rust
match read_port("aint_even_a_number_cmon_man") {
    Ok(port) => println!("port is {port}"),
    Err(PortErr::Parse { input, source }) => {
        println!("bad port {input:?}, parser says {source}");
    }
}
```

You can also hide that enum behind a public `#[error(transparent)]` newtype with a private field. The consumer doesn't have to know your parser type. [Opaque errors](https://docs.rs/thiserror/2.0.20/thiserror/#details)

## error-stack (0.8.0)

### Lazy

Give the operation a name, but we're using thiserror for the impls. [Docs](https://docs.rs/error-stack/0.8.0/error_stack/struct.Report.html)

The operation type is our choice here, same as exn and Er below. All three can also return the parser type directly and use bare `?`.

```rust
use error_stack::{Report, ResultExt};
use thiserror::Error;

#[derive(Debug, Error)] // thiserror
#[error("PortErr")]
pub struct PortErr;

pub fn read_port(input: &str) -> Result<u16, Report<PortErr>> {
    let port = input.parse::<u16>().change_context(PortErr)?;
    Ok(port)
}

println!("{error:?}");
```

```text
PortErr
├╴at examples/error_stack_lazy.rs:9:37
│
╰─▶ invalid digit found in string
    ╰╴at examples/error_stack_lazy.rs:9:37
```

No input, but we do get the location!

### With context

```rust
#[derive(Debug, Error)] // thiserror
#[error("invalid port {input:?}")]
pub struct PortErr {
    pub input: String,
}

pub fn read_port(input: &str) -> Result<u16, Report<PortErr>> {
    let port = input.parse::<u16>().change_context_lazy(|| PortErr {
        input: input.to_owned(),
    })?;
    Ok(port)
}

println!("{error:?}");
```

```text
invalid port "aint_even_a_number_cmon_man"
├╴at examples/error_stack_context.rs:10:37
│
╰─▶ invalid digit found in string
    ╰╴at examples/error_stack_context.rs:10:37
```

Add a field, fill it in when calling, no manual source field, the report keeps the old error.

You can also use 'Attachments' to add data.

`{}` only prints the current context, `{:?}` prints the report.

`source()` entries become text copies, those copies won't downcast to the original type but the error owning that source is still there.

I love the typed context and keeping the original errors. But you still need to make the error types yourself or use another derive, and `.change_context_lazy()` is a lot to type.

## Er (0.2)

(hey that's this one)

### Lazy

```rust
use er::*;

#[derive(Er)]
pub struct PortErr;

pub fn read_port(input: &str) -> Er<u16, PortErr> {
    let port: u16 = input.parse().er(())?;
    Ok(port)
}

println!("{}", error.er_report());
```

```text
PortErr @ examples/er_lazy.rs:7:35
`- invalid digit found in string
```

Exn 2 electric boogaloo, now with macros and less typing. Even an empty type gets us a name, a location and the original failure.

### With context

```rust
#[derive(Er)]
pub struct PortErr {
    pub input: String,
}

pub fn read_port(input: &str) -> Er<u16, PortErr> {
    let port: u16 = input.parse().er(|_| input)?;
    Ok(port)
}

println!("{}", error.er_report());
```

```text
PortErr { input: "aint_even_a_number_cmon_man" } @ examples/er_context.rs:8:35
`- invalid digit found in string
```

## Anyhow (1.0.104)

### Lazy

You don't even make errors. [Docs](https://docs.rs/anyhow/1.0.104/anyhow/)

```rust
use anyhow::Result;

pub fn read_port(input: &str) -> Result<u16> {
    let port = input.parse()?;
    Ok(port)
}

println!("{error:?}");
```

```text
invalid digit found in string
```

### With context

```rust
use anyhow::Context;

pub fn read_port(input: &str) -> Result<u16> {
    let port = input.parse().with_context(|| format!("invalid port {input:?}"))?;
    Ok(port)
}

println!("{error:?}");
```

```text
invalid port "aint_even_a_number_cmon_man"

Caused by:
    invalid digit found in string
```

Easy to add and easy to forget. The return type stays the same, every function above can keep doing `?` without adding anything.

Printing with `{}` or `.to_string()` only gives `invalid port "aint_even_a_number_cmon_man"`.

`{:#}` puts the causes on one line, `{:?}` gives the report above.

Backtraces can be enabled though. [Printing](https://docs.rs/anyhow/1.0.104/anyhow/struct.Error.html#display-representations)

You can still downcast to the original error, context can even be a struct. The return type just doesn't name your operation's error. [Context](https://docs.rs/anyhow/1.0.104/anyhow/trait.Context.html)

## SNAFU (0.9.2)

### Lazy

`Whatever`, no type to write. [Docs](https://docs.rs/snafu/0.9.2/snafu/)

```rust
use snafu::{Report, ResultExt, Whatever};

pub fn read_port(input: &str) -> Result<u16, Whatever> {
    input.parse().whatever_context("parse failed")
}

println!("{}", Report::from_error(error));
```

```text
parse failed

Caused by this error:
  1: invalid digit found in string
```

### With context

```rust
pub fn read_port(input: &str) -> Result<u16, Whatever> {
    input.parse().with_whatever_context(|_| format!("invalid port {input:?}"))
}

println!("{}", Report::from_error(error));
```

```text
invalid port "aint_even_a_number_cmon_man"

Caused by this error:
  1: invalid digit found in string
```

Pretty easy, but the input is text, not a field.

### Unique case

Or a small struct, SNAFU fills in the source and location:

```rust
use snafu::Snafu;
use std::num::ParseIntError;

#[derive(Debug, Snafu)]
#[snafu(display("invalid port {input:?} at {location}"))]
pub struct PortErr {
    pub input: String,
    pub source: ParseIntError,
    #[snafu(implicit)]
    pub location: snafu::Location,
}

pub fn read_port(input: &str) -> Result<u16, PortErr> {
    input.parse().context(PortSnafu { input })
}
```

```text
invalid port "aint_even_a_number_cmon_man" at examples/snafu_unique.rs:14:19

Caused by this error:
  1: invalid digit found in string
```

## rootcause (0.13.0)

### Lazy

`Report`, no type to write. [Docs](https://docs.rs/rootcause/0.13.0/rootcause/)

```rust
use rootcause::prelude::*;

pub fn read_port(input: &str) -> Result<u16, Report> {
    let port = input.parse::<u16>()?;
    Ok(port)
}

println!("{error}");
```

```text

 ● invalid digit found in string
 ╰ examples/rootcause_lazy.rs:4
```

No input, but we get the location!

### With context

```rust
use thiserror::Error;

#[derive(Debug, Error)] // thiserror
#[error("invalid port {input:?}")]
pub struct PortErr {
    pub input: String,
}

pub fn read_port(input: &str) -> Result<u16, Report<PortErr>> {
    let port = input.parse::<u16>().context_with(|| PortErr {
        input: input.to_owned(),
    })?;
    Ok(port)
}

println!("{error}");
```

```text

 ● invalid port "aint_even_a_number_cmon_man"
 ├ examples/rootcause_typed.rs:10
 │
 ● invalid digit found in string
 ╰ examples/rootcause_typed.rs:10
```

Now the return type requires `PortErr` as top, with the parser error kept as a sub error it can also use text context.

`{}` prints the report (other crates would do the report on {:?}), it has attachments, sub reports, lookup and shared cloning too, actually many many features. [Report](https://docs.rs/rootcause/0.13.0/rootcause/struct.Report.html)

It also has global hooks for creating and formatting reports. I'd rather put the data in a small error type than an attachment.

It's pretty good, the lazy version feels like anyhow with locations, and the typed version does require that top type, but you write those types yourself or use another derive.

### Unique case

Share the report without making `PortErr` implement clone:

```rust
let saved = error.into_cloneable();
let another = saved.clone();

println!("{}", saved.current_context().input);
println!("{}", another.current_context().input);
```

Same errors shared, not string copies. The shared report can't be mutated directly.

## exn (0.3.1)

Personal bias: i love exn

### Lazy

An empty error for the operation. thiserror supplies the impls here(you can also use derive_more, with manual impl Error), exn does the tree. [Docs](https://docs.rs/exn/0.3.1/exn/)

```rust
use exn::ResultExt;
use thiserror::Error;

#[derive(Debug, Error)] // thiserror
#[error("PortErr")]
pub struct PortErr;

pub fn read_port(input: &str) -> exn::Result<u16, PortErr> {
    let port = input.parse::<u16>().or_raise(|| PortErr)?;
    Ok(port)
}

println!("{error:?}");
```

```text
PortErr, at examples/exn_lazy.rs:9:37
`-- invalid digit found in string, at examples/exn_lazy.rs:9:37
```

This is what i like. Even without fields, the error has a name and a location.

### With context

```rust
#[derive(Debug, Error)] // thiserror
#[error("invalid port {input:?}")]
pub struct PortErr {
    pub input: String,
}

pub fn read_port(input: &str) -> exn::Result<u16, PortErr> {
    let port = input.parse::<u16>().or_raise(|| PortErr {
        input: input.to_owned(),
    })?;
    Ok(port)
}

println!("{error:?}");
```

```text
invalid port "aint_even_a_number_cmon_man", at examples/exn_context.rs:10:37
`-- invalid digit found in string, at examples/exn_context.rs:10:37
```

`{}` prints the top, `{:?}` prints the tree/report.

`raise_all` collects several failures (has to be the same type, and makes a new Exn (the tree/report type)).

Native sources get copied into another frame as text (like error-stack). Those frames can't be downcast to the original source, but the error owning it still can. [Construction](https://docs.rs/exn/0.3.1/exn/struct.Exn.html#method.new)

I love exn, but i got tired of typing `.or_raise()` and `.ok_or_raise()` everywhere. Exn doesn't make the types for you either, so these examples use thiserror. That little bit of friction is part of why i made Er.

### Exn vs Er

If we also want the input from our `PortErr` in the next error:

```rust
#[derive(Er)]
pub struct ConfigErr(pub String);

read_port(input).er_with(|t| ConfigErr::new(t.top.input.clone()))?;
```

Still keeps the old error and everything below it. A `map_err` that makes a fresh tree loses that history, which is really easy to do by accident.

`.er_find::<io::Error>()` finds the actual error, including inside native `source()` chains. `.er_find_all::<PortErr>()` gets all the bad ports.

Exn keeps the actual errors you give it too. BUT if a library gives you a `ReadErr` with an `io::Error` inside its `source()`, exn copies that source's message into another frame. So you can SEE the io error in the report, try to find its type in that frame and get nothing. [Exn construction](https://docs.rs/exn/0.3.1/exn/struct.Exn.html#method.new).

The real io error is still there in the normal `source()` of `ReadErr`. You don't even have to know it was a `ReadErr` to find it. But that means a find function has to check both the Exn frames AND the normal `source()` chains. [This one from the Exn issue](https://github.com/fast/exn/issues/65) only checks frames, so it misses the io error. `.er_find()` checks both.

`er_all!((), [a(), b()])` lets the results have different types. It runs all of them, keeps the failures and drops the oks. Exn's `raise_all` needs every error to convert to the same `Exn<T>` type.

Snapshots in Er for storing/having owned string version of the reports:

```rust
let saved = error.er_snapshot();
let json = serde_json::to_string(&saved).unwrap();

let saved: ErSnapshot = serde_json::from_str(&json).unwrap();
println!("{}", saved.er_report());
```

There's `.single_line()` and `for_each_line()` for printing too, mostly because logging multiline errors can be [complete shit](extra/systemd.md).

You don't HAVE to make an error type per function with Exn or Er. But i like distinct types because then the caller can't just `?` the error up unchanged. Er has a [`lazy` feature](lazy.md) if you don't want that.

## Eros (0.8.0-rc.1)

### Lazy

No type to write, no error set to list unless you want one. [Docs](https://docs.rs/eros/0.8.0-rc.1/eros/)

```rust
pub fn read_port(input: &str) -> eros::Result<u16> {
    let port = input.parse()?;
    Ok(port)
}

println!("{error:?}");
```

```text
invalid digit found in string

Backtrace (disabled):
```

### With string context

Let's name the parser type in the signature too:

```rust
use eros::Context;
use std::num::ParseIntError;

pub fn read_port(input: &str) -> eros::Result<u16, (ParseIntError,)> {
    input.parse::<u16>()
        .with_context(|| format!("invalid port {input:?}"))
}

println!("{error:?}");
```

```text
invalid digit found in string

  Context (innermost first):
    1. invalid port "aint_even_a_number_cmon_man"

Backtrace (disabled):
```

The caller sees `ParseIntError`, with context saved alongside it. You can erase the set or narrow it (which is cool and different to Er).

There's an optional `context` attribute over the whole function. You can use the arguments but not body locals. For 'this exact command failed', add `.with_context()` at the call. [Attribute](https://docs.rs/eros/0.8.0-rc.1/eros/attr.context.html)

### With typed context

```rust
#[derive(Debug, thiserror::Error)] // thiserror
#[error("command {command:?} on {machine}")]
pub struct CommandErr {
    pub command: String,
    pub machine: String,
}

pub fn read_port(input: &str) -> eros::Result<u16, (ParseIntError,)> {
    let command = format!("set-port {input}");
    input.parse::<u16>().with_context(|| {
        Box::new(CommandErr {
            command,
            machine: "ComputerKatten".to_owned(),
        }) as Box<dyn eros::SendSyncError>
    })
}

if let Some(command) = error.latest_context_error()
    .and_then(|e| e.as_any().downcast_ref::<CommandErr>()) {
    println!("{}", command.machine);
}
```

I don't love typing all that, but `CommandErr` is there with fields you can read, and the parser error is still typed as `ParseIntError`.

### Unique case: handle one error

Two possible error types, handle the bad number with a default and only io remains:

```rust
use eros::ReshapeUnion;
use std::io;

pub fn default_bad_port(
    result: eros::Result<u16, (ParseIntError, io::Error)>,
) -> eros::Result<u16, (io::Error,)> {
    result.recover::<ParseIntError, _>(|_| 85)
}
```

The parser error is no more, F in the chat.

`map_inner` lets us replace the inner error, if we decide to make our own type for it.

### Unique case: combine original errors

You can combine error types without making another enum, which i think is cool. The tuple names the possible types, not three errors all sitting there at runtime. `map_inner` can group them into your own error type if the caller needs your cases instead.

```rust
type FilePortErr = (io::Error, Utf8Error, ParseIntError);

fn file_port(path: &str) -> eros::Result<u16, FilePortErr> {
    let bytes = fs::read(path).union()?;
    let input = std::str::from_utf8(&bytes).union()?;
    read_port(input.trim()).widen()
}
```

The union gives the caller the original errors, not necessarily the cases you want them to handle. If you want your own typed context, you still have to make and map those types. The [tricky example](tricky-error-comparison.md#eros-080-rc1--thiserror-1) does that at the public boundary.

## Problemo (0.0.13)

### Lazy

```rust
use problemo::*;

pub fn read_port(input: &str) -> Result<u16, Problem> {
    let port = input.parse::<u16>()?;
    Ok(port)
}

println!("{error}");
```

```text
invalid digit found in string
```

### With context

```rust
use problemo::common::GlossError;

pub fn read_port(input: &str) -> Result<u16, Problem> {
    let port = input.parse::<u16>().map_via(|| GlossError::new(format!("invalid port {input:?}")))?;
    Ok(port)
}

println!("{error}");
```

```text
invalid port "aint_even_a_number_cmon_man": invalid digit found in string
```

`.via(MyErr)` adds your error above the current cause without source being needed, but it returns Problem, so the signature doesn’t name your top error type, but the actual errors are kept inside.

The distinctive thing is the caller chooses how failures are dealt with, pass a ProblemReceiver to something like a parser, then choose to collect errors, stop at the first one, or process them as they arrive.

`map_via` adds a cause, `GlossError` is its string error, no custom type. [Docs](https://docs.rs/problemo/0.0.13/problemo/)

It also has `.with(input.to_owned())` for typed attachments. Those aren't automatically printed, so this alone won't put `aint_even_a_number_cmon_man` in the output. You have to read the attachment yourself.

### Unique case

Same parser, caller chooses whether to collect errors or stop at the first one:

```rust
pub fn read_ports(inputs: &[&str], errors: &mut impl ProblemReceiver) -> Result<Vec<u16>, Problem> {
    let mut ports = vec![];
    for input in inputs {
        if let Some(port) = input.parse::<u16>().give_ok(errors)? {
            ports.push(port);
        }
    }
    Ok(ports)
}

let inputs = ["aint_even_a_number_cmon_man", "85", "fakenumber"];

let mut errors = Problems::default();
let ports = read_ports(&inputs, &mut errors)?; // [85] both failures saved
let collected = errors.check(); // Err don't forget this

let stopped = read_ports(&inputs, &mut FailFast); // Err on "aint_even_a_number_cmon_man"
```

Collecting 'Ok' can mean partial success, the errors are in the receiver, not Result.

## Nightly std::error::Report

(1.99.0-nightly)

Same two errors from thiserror, but remove `: {0}` / `: {source}` from their error text. Report prints the source:

```rust
#![feature(error_reporter)]

use std::error::Report;

println!("{}", Report::new(error).pretty(true));
```

Lazy:

```text
parse failed

Caused by:
      invalid digit found in string
```

With context:

```text
invalid port "aint_even_a_number_cmon_man"

Caused by:
      invalid digit found in string
```

Leave the source in Display and it prints twice, thats twice the value for half the price. Display prints it once, then Report follows `source()` and prints it again.

Report prints what you already have, it doesn't add context. Single line by default, `.pretty(true)` for this output.

[Nightly Docs](https://doc.rust-lang.org/std/error/struct.Report.html)

## Others

[terrors](https://docs.rs/terrors/0.3.3/terrors/) is closer to Eros, with a set of possible errors where you can handle one and pass the rest up. [error_set](https://docs.rs/error_set/0.9.2/error_set/) actually makes enums with your own variants, and lets you combine smaller sets. I haven't tried either in the tricky example.

[lazy_errors](https://docs.rs/lazy_errors/latest/lazy_errors/) is worth a look if collecting several failures and you want to keep going and collect the failures, including errors from cleanup. It has nesting and locations too, aggregation focus is interesting.

