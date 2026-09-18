# Simple Error Comparison ™

## Before we start

DISCLAIMER: this is obviously the error case i care about, and its biased, but i try to be fair.

Links on this page: [thiserror](#thiserror-2020), [Anyhow](#anyhow-10104), [SNAFU](#snafu-092), [error-stack](#error-stack-080), [rootcause](#rootcause-0130),
[exn](#exn-031), [Eros](#eros-080-rc1), [Problemo](#problemo-0013), [Nightly std::error::Report](#nightly-stderrorreport), [Others](#others), [Er](#er-020)

For a tricky example check out the [tricky comparison](tricky-error-comparison.md) after reading this (foreign errors, own errors, the original error, string context, typed context, using / consuming, public boundary).

## The quest /task we're about to do

Someone entered `aint_even_a_number_cmon_man` as input and we get this:

```text
invalid digit found in string
```

So lets call `read_port("aint_even_a_number_cmon_man")` and let's do 2 versions...

One lazy try where we do whatever the library makes easy (and lets be honest, its what people end up doing).

Then a second run where we add context (would be nice to see what went wrong, and no... logging is not the same).

Done on Rust 1.98.1

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

An exn maintainer `_tison` wrote: `"The real trigger is I'd prefer or_raise over change_context_lazy very much, lol"` from [Reddit link](https://www.reddit.com/r/rust/comments/1qs68cn/comment/o2wtdwq/) where he also says errorstacks code and error chains are overcomplicated.

Bro i just want to use crate_name::*; and do the same thing everywhere. And hot take, error handling is a huge part of most apps, needs to be easy to type, and i dont want to rely on snippets or ai, or huge proc macros spanning the entire function (The error type is fine imo).

And the moment you add tiny friction on making errors, people aren't gonna want to do it. 

When trying to convince other people of how great exn was, the examples aren't the easiest and it's confusing for people that you're doing std::Result<T, Exn<E>>, and people immediately wanna do .map_err(||) and ruin that poor error reports for good.

### Exn vs Er

If we also want the input from our `PortErr` in the next error:

```rust
#[derive(Er)]
pub struct ConfigErr(pub String);

read_port(input).er_with(|t| t.top.input.clone())?;
```

Still keeps the old error and everything below it. Exn can do this with `map_err` then `.raise()`, but here it's very easy to destroy your error tree and now I'm manually having to worry and do it.

`.er_find::<io::Error>()` finds the actual error, including inside native `source()` chains. `.er_find_all::<PortErr>()` gets all the bad ports.

Exn keeps the actual errors you give it too. BUT if a library gives you a `ReadError` with an `io::Error` inside its `source()`, exn copies that source's message into a child frame. So you can SEE the io error in the report, try to find its type in that frame and get nothing. [Exn construction](https://docs.rs/exn/0.3.1/exn/struct.Exn.html#method.new).

The real io error is still there in the normal `source()` of `ReadError`. You don't even have to know it was a `ReadError` to find it. But that means a find function has to check both the Exn frames AND the normal `source()` chains. [This one from the Exn issue](https://github.com/fast/exn/issues/65) only checks frames, so it misses the io error. `.er_find()` checks both.

`er_all!((), [a(), b()])` lets the results have different types. It runs all of them, keeps the failures and drops the oks. Exn's `raise_all` needs the children to convert to the same `Exn<T>` type.

Snapshots in Er for storing/having owned string version of the reports:

```rust
let saved = error.er_snapshot();
let json = serde_json::to_string(&saved).unwrap();

let saved: ErSnapshot = serde_json::from_str(&json).unwrap();
println!("{}", saved.er_report());
```

There's `.single_line()` and `for_each_line()` for printing too, mostly because logging multiline errors can be [complete shit](systemd.md).

And my most hot take:

I also think not having convenient macros/helpers for making the error types will result in you 'not bothering' with having the types where it makes sense, and you might end up lazily abusing the non generic version.

In Exn and Er you don't HAVE to have an error type per function, you can have it per section or whatever but... I still think that actually having the error typed is crucial, and i think that by allowing users to '?' it up lazily, the point is gone. It's also why i don't think Er should have an ErLazy (like ErTest) for normal application code.

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
invalid digit found in string

  Context (innermost first):
    1. invalid port "aint_even_a_number_cmon_man"

Backtrace (disabled):
```

Trailing comma to make it a tuple(since there's only one value).

The caller sees `ParseIntError`, with context saved alongside it. You can erase the set or narrow it (which is cool and different to Er).

There's an optional `context` attribute over the whole function, pretty spicy. You pick the arguments to format, but can't use body locals there. For 'this exact command failed', add `.with_context()` at the call. (I would always want that.) [Attribute](https://docs.rs/eros/0.8.0-rc.1/eros/attr.context.html)

### With typed context

```rust
#[derive(Debug, thiserror::Error)] // thiserror
#[error("command {command:?} on {machine}")]
pub struct CommandError {
    pub command: String,
    pub machine: String,
}

pub fn read_port(input: &str) -> eros::Result<u16, (ParseIntError,)> {
    let command = format!("set-port {input}");
    input.parse::<u16>().with_context(|| {
        Box::new(CommandError {
            command,
            machine: "ComputerKatten".to_owned(),
        }) as Box<dyn eros::SendSyncError>
    })
}

if let Some(command) = error.latest_context_error()
    .and_then(|e| e.as_any().downcast_ref::<CommandError>()) {
    println!("{}", command.machine);
}
```

Personally i don't enjoy the ergonomics of this, but it still has some cool things like `CommandError` being there and you can read the fields, the parser error is kept too, and the set still says `ParseIntError`.

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

You can combine error types without making another enum, which i think is cool. Even with a type alias, that's way less work than making an enum. The alias just names the set. If you stop there, your caller gets the original error types, not the cases you'd name for them. In 0.8, `map_inner` can group them into your own error type, if you want to write that type and do the mapping.

Lets say a function can fail with 3 different errors. The tuple names the possible types. At runtime Eros holds one of them, not a tuple with all 3 errors inside it.

```rust
type FilePortError = (io::Error, Utf8Error, ParseIntError);

fn file_port(path: &str) -> eros::Result<u16, FilePortError> {
    let bytes = fs::read(path).union()?;
    let input = std::str::from_utf8(&bytes).union()?;
    read_port(input.trim()).widen()
}
```

Now the angle here is we aren't handling the error or composing it for our caller, we're just providing our caller with 'what could have gone wrong' which in my opinion is 'just thiserror but you dont make the enum everytime', so yes it avoids the pyramid, but it still making a 'stepped pyramid' and not handling/providing our caller with convenience.

I think eros is great for the use case where you 'throw together the actual types of the errors you got', but i feel like it gets messy once you want custom error types with typed context. The [tricky example](tricky-error-comparison.md#eros-080-rc1--thiserror-1) shows the union and context, then separately makes a plain public error.

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

`.via(MyErr)` adds your error above the current cause without source being needed.

But it returns Problem, so the signature doesn’t name your top error type, but the actual errors are kept inside.

The distinctive thing is the caller chooses how failures are dealt with, pass a ProblemReceiver to something like a parser, then choose to collect errors, stop at the first one, or process them as they arrive.

`map_via` adds a cause, `GlossError` is its string error. No custom type needed here. [Docs](https://docs.rs/problemo/0.0.13/problemo/)

It also has `.with(input.to_owned())` for typed attachments. Those aren't automatically printed, so this alone won't put `aint_even_a_number_cmon_man` in the output. You have to read the attachment yourself.

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

Leave the source in Display and it prints twice, thats twice the value for half the price. Display printed it once, then Report followed `source()` and printed it again.

Treats debug and display the same. It seems like the secret meanings behind random text formats that suddenly gained sentient meanings were left behind (Which i think is a good move). 

Report prints what you already have, it doesn't add context. Single line by default, `.pretty(true)` for this output. 

[Nightly Docs](https://doc.rust-lang.org/std/error/struct.Report.html)

## Others 

[terrors](https://docs.rs/terrors/0.3.3/terrors/) is closer to Eros, with a set of possible errors where you can handle one and pass the rest up. [error_set](https://docs.rs/error_set/0.9.2/error_set/) actually makes enums with your own variants, and lets you combine smaller sets. I haven't tried either in the tricky example.

[lazy_errors](https://docs.rs/lazy_errors/latest/lazy_errors/) is worth a look if collecting several failures and you want to keep going and collect the failures, including errors from cleanup. It has nesting and locations too, aggregation focus is interesting.

## Er (0.2.0)

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

Exn 2 electric boogalo now with macros and shortened syntax (and some internal changes that has ramifications), but like exn an empty type still gets us a name, a location and the original failure.

### With context

```rust
#[derive(Er)]
pub struct PortErr {
    pub input: String,
}
pub fn read_port(input: &str) -> Er<u16, PortErr> {
    let port: u16 = input.parse().er(|| input)?;
    Ok(port)
}

println!("{}", error.er_report());
```

```text
PortErr { input: "aint_even_a_number_cmon_man" } @ examples/er_context.rs:8:35
`- invalid digit found in string
```

# Er weird stuff

Returning `Er<T, E>` means the caller is in Er world now. On public boundaries make a normal error and convert. [Checkout the example](examples.md#public-error).

The tree has no Display, Debug, or Error. Pick `.er_report()` or `.er_top()`, then Display and Debug do the same. (Designed this way, to avoid mistakes).

No backtraces (i prefer explicit context, hot take i know).

No cloning the live tree built in. Snapshots can be cloned, but save text and structure, not the original error types.

## Biased?

Yes and obviously i only care about a subset of error handling in rust.

Thanks for reading, and if you don't agree with me that's probably good, I have some stupid opinions.
