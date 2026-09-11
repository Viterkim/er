# Different error library comparisons

## The quest we're about to go on

Done on Rust 1.98.1

Someone entered `aint_even_a_number_cmon_man` as input and we get this:

```text
invalid digit found in string
```

So lets call `read_port("aint_even_a_number_cmon_man")` and let's do 2 versions...

One lazy try where we do whatever the library makes easy (and lets be honest, its what people end up doing).

Then a second run where we add context (would be nice to see what went wrong, and no... logging is not the same).

DISCLAIMER: this is obviously the error case i care about, and its biased, but i try to be fair.

## thiserror (2.0.20)

Helper macro for doing `From<E>`, `Error` and `Display`. [Docs](https://docs.rs/thiserror/2.0.20/thiserror/)

### Lazy

An enum of the things that can fail, then just `?`.

```rust
use std::num::ParseIntError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PortError {
    #[error("parse failed: {0}")]
    Parse(#[from] ParseIntError),
}
pub fn read_port(input: &str) -> Result<u16, PortError> {
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
pub enum PortError {
    #[error("invalid port {input:?}: {source}")]
    Parse {
        input: String,
        source: ParseIntError,
    },
}
pub fn read_port(input: &str) -> Result<u16, PortError> {
    let port = input.parse().map_err(|source| PortError::Parse {
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

There goes `#[from]`, it gets the parser error, not the local `input`, so we gotta get down and dirty and type stuff out.

This is like, the opposite of what you thought the library would do when you got it the first time. `?` is too easy, and the context variant is annoying.

No report, but thats fair enough i think the bigger annoyance with this enum is your consumer is now caring about what exact errors you got, and not what you designed for them.

### Unique case

The upside of declaring the source, the caller can just take it out. No search or downcast. Same goes for SNAFU's typed errors below.

```rust
match read_port("aint_even_a_number_cmon_man") {
    Ok(port) => println!("port is {port}"),
    Err(PortError::Parse { input, source }) => {
        println!("bad port {input:?}, parser says {source}");
    }
}
```

You can also hide that enum behind a public `#[error(transparent)]` newtype with a private field. The consumer doesn't have to know your parser type. [Opaque errors](https://docs.rs/thiserror/2.0.20/thiserror/#details)

## Anyhow (1.0.104)

### Lazy

No error type to write, pretty based. [Docs](https://docs.rs/anyhow/1.0.104/anyhow/)

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

`{:#}` puts the causes on one line.

`{:?}` gives the report above.

Backtraces can be enabled though. [Printing](https://docs.rs/anyhow/1.0.104/anyhow/struct.Error.html#display-representations)

You can still downcast to the original error, context can be even be struct. The return type just doesn't name your operation's error. [Context](https://docs.rs/anyhow/1.0.104/anyhow/trait.Context.html)

Also `anyhow::Error` doesn't impl Error because of a trait overlap or its gonna break its generic From<E> conversion. (ER DOES THE SAME SHIT with tree/report/top, `.opaque_err()` is the explicit way out where you then can't search the sub errors).

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
pub struct PortError {
    pub input: String,
    pub source: ParseIntError,
    #[snafu(implicit)]
    pub location: snafu::Location,
}
pub fn read_port(input: &str) -> Result<u16, PortError> {
    input.parse().context(PortSnafu { input })
}
```

```text
invalid port "aint_even_a_number_cmon_man" at examples/snafu_unique.rs:14:19

Caused by this error:
  1: invalid digit found in string
```

`PortSnafu` is the generated selector, not the error. Still more declaration than i want, but no manual source wiring. [Selectors](https://docs.rs/snafu/0.9.2/snafu/derive.Snafu.html), [Location](https://docs.rs/snafu/0.9.2/snafu/struct.Location.html)

I'm gonna be honest, pretty cool name but it aint it for me.

## error-stack (0.8.0)

### Lazy

Give the operation a name, but we're using thiserror for the impls. [Docs](https://docs.rs/error-stack/0.8.0/error_stack/struct.Report.html)

The operation type is our choice here, same as exn and Er below. All three can also return the parser type directly and use bare `?`.

```rust
use error_stack::{Report, ResultExt};
use thiserror::Error;

#[derive(Debug, Error)] // thiserror
#[error("PortError")]
pub struct PortError;

pub fn read_port(input: &str) -> Result<u16, Report<PortError>> {
    let port = input.parse::<u16>().change_context(PortError)?;
    Ok(port)
}

println!("{error:?}");
```

```text
PortError
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
pub struct PortError {
    pub input: String,
}
pub fn read_port(input: &str) -> Result<u16, Report<PortError>> {
    let port = input.parse::<u16>().change_context_lazy(|| PortError {
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

Add a field, fill it in when calling, no manual source field, the report keeps the old error. VERY NICE!

You can also use 'Attachments' to add data.

`{}` only prints the current context.

`{:?}` prints the report. `expand` and `push` collect several failures. [Report](https://docs.rs/error-stack/0.8.0/error_stack/struct.Report.html)

`source()` entries become text copies, those copies won't downcast to the original type but the error owning that source is still there.

Your own contexts keep their types. [Source contexts](https://docs.rs/error-stack/0.8.0/src/error_stack/context.rs.html)

It also has global formatting hooks for attachments(like rootcause). Personally i want the data in the error type and format it there.

I love the typed context and keeping the original errors, that's pretty nice!!. But you still need to make the error types yourself or use another derive, and change_context_lazy is a lot of typing for my small fingers.

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
pub struct PortError {
    pub input: String,
}
pub fn read_port(input: &str) -> Result<u16, Report<PortError>> {
    let port = input.parse::<u16>().context_with(|| PortError {
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

Now the return type requires `PortError` as top, with the parser error kept as a sub error it can also use text context.

`{}` prints the report (other crates would do the report on {:?}), it has attachments, sub reports, lookup and shared cloning too, actually many many features. [Report](https://docs.rs/rootcause/0.13.0/rootcause/struct.Report.html)

It also has global hooks for creating and formatting reports, operation context attachments feel like a band aid to me, why not put the data in a small error type. 

It's pretty good, the lazy version feels like anyhow with locations, and the typed version does require that top type, but you write those types yourself or use another derive.

### Unique case

Share the report without making `PortError` implement clone:

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
#[error("PortError")]
pub struct PortError;

pub fn read_port(input: &str) -> exn::Result<u16, PortError> {
    let port = input.parse::<u16>().or_raise(|| PortError)?;
    Ok(port)
}

println!("{error:?}");
```

```text
PortError, at examples/exn_lazy.rs:9:37
`-- invalid digit found in string, at examples/exn_lazy.rs:9:37
```

This is what i like, even when we can't be bothered adding fields we atleast have a unique type(forced to add context a layer up if they also have a another type) AND there's a name and a location.

### With context

```rust
#[derive(Debug, Error)] // thiserror
#[error("invalid port {input:?}")]
pub struct PortError {
    pub input: String,
}
pub fn read_port(input: &str) -> exn::Result<u16, PortError> {
    let port = input.parse::<u16>().or_raise(|| PortError {
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

Same but the diff to add input is so small so might as well do it when it makes sense, no source field to type out etc.

`{}` prints the top.

`{:?}` prints the tree/report.

`raise_all` collects several failures (has to be the same type, and makes a new Exn (the tree/report type)).

Native sources get copied into child frames as text(like error-stack), those copies lose downcasting, BUT the error owning the source can still do it [Construction](https://docs.rs/exn/0.3.1/exn/struct.Exn.html#method.new)

Ok extended opinion piece inc:

I love exn, but i got tired of typing .or_raise() everywhere, ok_or_raise(), no macros to help creating types. 

It's also funny, because exn came about because they were tired of error-stack and typing `change_context_lazy(||)` and i aint gonna lie man `.ok_or_raise(||)` for options is the same lol. 

'_tison' from exn wrote `"The real trigger is I'd prefer or_raise over change_context_lazy very much, lol"` from [Reddit link](https://www.reddit.com/r/rust/comments/1qs68cn/comment/o2wtdwq/) where he also says errorstacks code and error chains are overcomplicated.

Bro i just want to use crate_name::*; and do the same thing everywhere. And hot take, error handling is a huge part of most apps, needs to be easy to type, and i dont want to rely on snippets or ai, or huge proc macros spanning the entire function (The error type is fine imo).

And the moment you add tiny friction on making errors, people aren't gonna want to do it. 

When trying to convince other people of how great exn was, the examples aren't the easiest and it's confusing for people that you're doing std::Result<T, Exn<E>>, and people immediately wanna do .map_err(||) and ruin that poor error reports for good.

I really really like exn, but theres small things that build up over time...

## Eros (0.7.0)

### Lazy

No type to write, no error set to list unless you want one. [Docs](https://docs.rs/eros/0.7.0/eros/)

```rust
pub fn read_port(input: &str) -> eros::Result<u16> {
    let port = input.parse()?;
    Ok(port)
}

println!("{error:?}");
```

```text
ParseIntError { kind: InvalidDigit }
---
```

### With context

Lets name the parser type in the signature too:

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
ParseIntError { kind: InvalidDigit }
---

Context:
	- invalid port "aint_even_a_number_cmon_man"

---
```

Trailing comma to make it a tuple. Reminds me of OCaml a bit variadic ish.

The caller sees `ParseIntError`, with context saved alongside it. You can erase the set or narrow it (thats so cool!).

There's an optional `#[context(...)]` over the whole function, pretty spicy. You pick the arguments to format, but can't use body locals there. For 'this exact command failed', add `.with_context()` at the call. (I would always want that.) [Attribute](https://docs.rs/eros/0.7.0/eros/attr.context.html)

`location` adds callsites; backtrace support is on by default. [Features](https://docs.rs/crate/eros/0.7.0/features)

I will say i think eros is very unique, i think it's cool with different features and trying out stuff. 

### Unique case: handle one type

Two possible error types, handle the bad number with a default and only io remains in the set:

```rust
use eros::ReshapeUnion;
use std::io;

pub fn default_bad_port(
    result: eros::Result<u16, (ParseIntError, io::Error)>,
) -> eros::Result<u16, (io::Error,)> {
    match result.narrow::<ParseIntError, _>() {
        Ok(_) => Ok(85),
        Err(rest) => rest,
    }
}
```

The parser error is no more, F in the chat.

`Err(rest)` means it aint a parser error, `rest` is still a Result (can be ok).

Er doesn't do this. A type per function isn't the same as proving which failures remain. [Narrowing](https://docs.rs/eros/0.7.0/eros/trait.ReshapeUnion.html)

### Unique case: context can be a type too

```rust
#[derive(Debug, thiserror::Error)] // thiserror
#[error("command {command:?} on {machine}")]
pub struct CommandEr {
    pub command: String,
    pub machine: String,
}

pub fn read_port(input: &str) -> eros::Result<u16, (ParseIntError,)> {
    let command = format!("set-port {input}");
    input.parse::<u16>().with_context(|| {
        Box::new(CommandEr {
            command,
            machine: "ComputerKatten".to_owned(),
        }) as Box<dyn eros::SendSyncError>
    })
}

if let Some(command) = error.latest_error().as_any().downcast_ref::<CommandEr>() {
    println!("{}", command.machine);
}
```

Thats the actual `CommandEr`, you can read its fields. The parser error is kept too, and the set still says `ParseIntError`. [Context storage](https://docs.rs/eros/0.7.0/src/eros/context.rs.html)

`latest_error()` skips text and gets the newest error context, or the original error. Not a search through them all. I still want the operation type in the signature.

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

`.via(MyError)` adds your error above the current cause without source being needed.

But it returns Problem, so the signature doesn’t name your top error type, but the actual errors are kept inside.

The distinctive thing is the caller chooses how failures are dealt with, pass a ProblemReceiver to something like a parser, then choose to collect errors, stop at the first one, or process them as they arrive.

`map_via` adds a cause, `GlossError` is its string error. No custom type needed here. [Docs](https://docs.rs/problemo/0.0.13/problemo/)

It also has `.with(...)` for typed attachments. Those aren't automatically printed, adding `.with(input.to_owned())` alone won't put `aint_even_a_number_cmon_man` in this output. You have to read the attachment yourself.

I like the focus on aggregation/stopping.

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

Same two errors from thiserror, but remove `: {0}` / `: {source}` from their `#[error(...)]` text. Report prints the source:

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

Leave the source in Display and it prints twice, thats twice the value for half the price. Display printed it once, then Report followed `source()` and printed it again.

Treats debug and display the same. It seems like the secret meanings behind random text formats that suddenly gained sentient meanings were left behind (Which i think is a good move). 

Report prints what you already have, it doesn't add context. Single line by default, `.pretty(true)` for this output. 

[Nightly Docs](https://doc.rust-lang.org/std/error/struct.Report.html)

## Others 

Set variants [terrors](https://docs.rs/terrors/0.3.3/terrors/) and [error_set](https://docs.rs/error_set/0.9.2/error_set/) are kinda like eros, useful when callers want to handle those types directly, similar to EROS and doesn't solve the case i care about.

[lazy_errors](https://docs.rs/lazy_errors/latest/lazy_errors/) is worth a look if collecting several failures and you want to keep going and collect the failures, including errors from cleanup. It has nesting and locations too, pretty useful if aggregation is what you care about the most.

## Er (0.1.0)

(hey that's this one)

### Lazy

```rust
use er::*;

#[derive(Er)]
pub struct PortEr;

pub fn read_port(input: &str) -> Er<u16, PortEr> {
    let port: u16 = input.parse().er(PortEr::new)?;
    Ok(port)
}

println!("{}", error.er_report());
```

```text
PortEr @ examples/er_lazy.rs:7:35
`- invalid digit found in string @ examples/er_lazy.rs:7:35
```

Exn 2 electric boogalo now with macros and shortened syntax (and some internal changes that has ramifications), but like exn an empty type still gets us a name, a location and the original failure.

### With context

```rust
#[derive(Er)]
pub struct PortEr {
    pub input: String,
}
pub fn read_port(input: &str) -> Er<u16, PortEr> {
    let port: u16 = input.parse().er(|| PortEr::new(input))?;
    Ok(port)
}

println!("{}", error.er_report());
```

```text
PortEr { input: "aint_even_a_number_cmon_man" } @ examples/er_context.rs:8:35
`- invalid digit found in string @ examples/er_context.rs:8:35
```

Now there's some bullshit you also have to learn for Er (some for good reason).

Returning `Er<T, E>` means the caller is in Er world now. On public boundries make a normal error and convert. [Checkout the example](examples.md#public-error).

The tree has no Display, Debug, or Error. Pick `.er_report()` or `.er_top()`, then Display and Debug do the same. (Designed this way, to avoid mistakes).

Local data is fine in the root. Moving it into a boxed child needs `Send + Sync + 'static`.

No backtraces (i prefer explicit context, hot take i know).

No cloning the live tree built in. Snapshots can be cloned, but save text and structure, not the original error types.

`.opaque_err()` gives you a standard Error but hides the tree from ordinary error finds.

If you NEED Error on the Wrap itself, [Wrap with `std_error`](macros.md#wrap-with-error) does that, and needs `.er_from_wrap(...)` on the way back. This is without a doubt the worst thing about Er, but i can't come up with anything better, i hope you will never need it and can forgive me. (ONLY used for implementing a foreign trait on a wrapper which NEEDS to implement `Error` itself).

If a foreign error prints its source AND returns it from `source()`, the report can repeat that text. Er doesn't guess which bits to remove. [Standard Error guidance](https://doc.rust-lang.org/std/error/trait.Error.html#error-source).

## Biased?

Yes and obviously i only care about a subset of error handling in rust.

Thanks for reading, and if you don't agree with me that's probably good, I have some stupid opinions.
