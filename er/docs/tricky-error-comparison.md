# Tricky Error Comparison - Overview

DISCLAIMER: this is a made up error case to try to hit the pain points of different error handling libraries (including Er), I've tried to hit complexity with composition while still making it as readable as possible for the examples. Again I'm obviously biased, but i tried my best.

Read the [simple comparison](simple-error-comparison.md) first (basic usage, context, output).

### Links - Sections

[Printed errors](#what-gets-printed)

[Original errors](#getting-the-original-errors)

[Public errors](#public-errors)

### Links - Libraries

[thiserror, lazy style](#thiserror-lazy-style-2020-1)

[thiserror, verbose style](#thiserror-verbose-style-2020-1)

[Anyhow](#anyhow-10104-1)

[SNAFU](#snafu-092-1)

[error-stack](#error-stack-080--thiserror-1)

[rootcause](#rootcause-0130--thiserror-1)

[exn](#exn-031--thiserror-1)

[Er](#er-020-1)

[Eros](#eros-080-rc1--thiserror-1)

[Problemo](#problemo-0013-1)

`(+ thiserror)` means i used its derive to write `Display` and `Error`.

## The quest / task we're about to do

We will try to listen, in this example we just have `127.0.0.1:8080` (but imagine its dynamic).

A bad Ip or port we'll group to bad input. (Two parser errors we turn into one `InvalidInput`, so grouping into what the user cares about, while still keeping the original error in some cases).

Port `85` is sacred, and not allowed. (An error we made up ourselves, so nothing to map from).

If bind fails, we'll go looking for other available ports and grab `kind()` from the io error. If the address is already in use, the caller can show the ports we found (digging for info in the error below).

Then we'll try another thing. Say a dependency gives us an error with an io error inside it. Can we get both actual errors back, or do we just get their messages?

And if this is a public api, can someone handle our error without having to use the same error library we did?

Done on Rust 1.98.1

## Opinion

The hottest take from me is that each library (because of ergonomics and inner workings) forces you to do a thing in a certain way. The way each works makes you structure errors fundamentally different, and if you think it's about "using it correctly", i don't think you are being honest.

In some cases you COULD make them more similar, but you're fighting an uphill battle. Er/Exn makes you always design your errors, Eros makes you narrow/widen what inner errors could occur, and thiserror lets you `?` the original errors or makes you map them into your own.

My biggest point is, why is the most inner error sacred/something your caller above you should ever handle/know? As long as you have the original message, i think that's exactly what you need, and if original context is there with something like an error code, that should be brought up into your own type.

If your caller needs to do stuff with variants, your caller should not get a pyramid of nested chained types, they should be getting 1 type of what happened they can match on. And if they don't care about what happened they should just be forced (at a type level) to yeet it up one layer so the next layer also adds the call location(which is why i don't think you should have an AppErr in Er, so you can just '?' it at every step).

I don't think you should ever give a serde error, or an io error to someone else, you should give them a typed error that is handled or set up for handling. I think people's resistance to this is just because they are used to it.

## TLDR (libraries compared, pain points, experience)

### thiserror, lazy style (2.0.20)

One variant per error we got, then `?` works and the originals stay typed. We don't get the `input`, address or available ports though, and bad IP / bad port are separate cases instead of one `InvalidInput`. This is the thiserror pyramid people actually write because it's easy, then at night you get a 'file not found' and you have no clue where. (yes you can add context, do people do it everywhere? no, or well maybe they do but then their code base is 80% that).

### thiserror, verbose style (2.0.20)

Now we make the cases around what our caller cares about and keep the original errors too. `InvalidInput` needs another error enum underneath because IP and port parsing are different types. We also write every `map_err`, source field and bit of context ourselves.

### Anyhow (1.0.104)

`?` everything and add string context where it matters. It's short, the report is good and the original errors stay there for downcasting. Our `InvalidInput`, sacred port and available ports are just text though, so the caller can't match on them or pass the ports to `show_available_ports`.

### SNAFU (0.9.2)

We get an enum with typed source fields, and its context callback can see the failed error. That makes the bind case pretty nice here. To put both parser errors under one `InvalidInput`, we box them, so now you gotta downcast the source to know which parser failed. I think it's fine and SNAFU can 'technically' cover most cases, i just think that... I'm not gonna type all that.

### Eros (0.8.0-rc.1) (+ thiserror)

Eros gives us a union of the original error types. That's nice if those types are what the caller wants. Here bad Ip and bad port stay different types even though they're both bad input to us. The ports sit in separate context, so matching the io error doesn't guarantee we have them. We can map into our own errors, but then we have to make and map them ourselves. It avoids the pyramid but it has the same philosophy of caring about the original errors type and yeeting that up that i don't agree with. 

### error-stack (0.8.0) (+ thiserror)

We get a typed current error with the old errors kept in a report. But its lazy context callback can't see the failed error, so bind needs `map_err`. Its lookup finds the error we gave it, but misses the real io error inside that error. That nested error was copied into a frame as text. (i feel like its like exn but with a worse user experience)

### rootcause (0.13.0) (+ thiserror)

We get a typed current error with the old error kept underneath. Reading `kind()` while adding context takes `map_err`. Finding an error inside another one takes some digging through reports and sources.

### exn (0.3.1) (+ thiserror)

We get a typed top error and a tree underneath. Exn keeps the old errors, but has no built-in type search. It also copies native sources into frames as text, so searching just the frames still misses the real error inside another one. Reading `kind()` at bind needs `map_err`. I think exn is very close to being insanely good (which is why i made Er).

### Er (0.2.0)

We get one typed error with the stuff our caller cares about. `.er_with()` can read the failed error and keep it underneath. Everything below `.top` needs a search. AND `.er()` can't put that old error in a typed source field on the enum variant as well (like a classic thiserror enum) because the tree owns it (this is the biggest limitation, which i don't think is a negative, i don't think you should be doing that, and it applies to all the report styled error handling crates anyway).

### Problemo (0.0.13)

Tags say what happened and attachments hold the data. The caller has to look up both, and finding a tag doesn't even guarantee its attachment is there. Problemo can find errors inside normal `source()` chains. Reading `kind()` at bind still needs `map_err`.

# What gets printed

First lets look at what actually comes out, `127.0.0.1:nope` is entered and the port parser only says `invalid digit found in string`, so what happens? (keep reading, you don't even have to wait until the next episode).

```rust
let error = listen("127.0.0.1:nope").unwrap_err();
```

## Er

```rust
println!("{}", error.er_report());
```

```text
ListenErr::InvalidInput { input: "127.0.0.1:nope" } @ src/bin/er.rs:24:36
`- invalid digit found in string
```

## Eros

```rust
println!("{error:?}");
```

```text
invalid digit found in string

  Context (innermost first):
    1. bad input: 127.0.0.1:nope

Backtrace (disabled):
```

## thiserror, lazy style

```rust
println!("{error:?}");
```

```text
InvalidPort(ParseIntError { kind: InvalidDigit })
```

The parser error is still there, but the input isn't.

## thiserror, verbose style

```rust
println!("{error:?}");
```

```text
InvalidInput { input: "127.0.0.1:nope", source: Port(ParseIntError { kind: InvalidDigit }) }
```

Now both are there. Getting that one `InvalidInput` case takes the extra error enum in the full example.

## Anyhow

```rust
println!("{error:?}");
```

```text
bad input: 127.0.0.1:nope

Caused by:
    invalid digit found in string
```

## SNAFU

```rust
println!("{}", snafu::Report::from_error(error));
```

```text
bad input: 127.0.0.1:nope

Caused by this error:
  1: invalid digit found in string
```

## error-stack

```rust
println!("{error:?}");
```

```text
bad input: 127.0.0.1:nope
├╴at src/bin/error-stack.rs:29:10
│
╰─▶ invalid digit found in string
    ╰╴at src/bin/error-stack.rs:29:10
```

## rootcause

```rust
println!("{error}");
```

```text

 ● bad input: 127.0.0.1:nope
 ├ src/bin/rootcause.rs:29
 │
 ● invalid digit found in string
 ╰ src/bin/rootcause.rs:29
```

## exn

```rust
println!("{error:?}");
```

```text
bad input: 127.0.0.1:nope, at src/bin/exn.rs:29:10
`-- invalid digit found in string, at src/bin/exn.rs:29:10
```

## Problemo

```rust
println!("{error}");
```

```text
bad input: invalid digit found in string
```

The input is still in the `Input` attachment. This printout doesn't show it.

# Tricky Error Comparison - Full Examples

## Shared functions

Let's imagine `find_available_ports(address: SocketAddrV4) -> Vec<u16>` finds ports to suggest, `show_available_ports(ports: &[u16]) -> ()` shows them, and `start_server(listener: TcpListener) -> ()` starts the server.

## thiserror, lazy style (2.0.20)

One variant for each error we get, then `?` does the rest. (I know i have done this many many times).

```rust
// Making it
#[derive(Debug, thiserror::Error)]
pub enum ListenError {
    #[error("invalid IP: {0}")]
    InvalidIp(#[from] AddrParseError),
    #[error("invalid port: {0}")]
    InvalidPort(#[from] ParseIntError),
    #[error("port 85 is sacred")]
    SacredPort,
    #[error("couldn't bind: {0}")]
    Bind(#[from] io::Error),
}

pub fn listen(input: &str) -> Result<TcpListener, ListenError> {
    // Normal stuff
    let (ip, port) = input.split_once(':').unwrap_or((input, ""));

    // First 2 errors, `?` picks their variant
    let ip = ip.parse::<Ipv4Addr>()?;
    let port = port.parse::<u16>()?;

    // Third error, our own rule that port 85 is sacred
    if port == 85 {
        return Err(ListenError::SacredPort);
    }

    // Fourth error, another `?`
    let address = SocketAddrV4::new(ip, port);
    Ok(TcpListener::bind(address)?)
}

// Using it
fn main() {
    let input = "127.0.0.1:8080";
    match listen(input) {
        Ok(listener) => start_server(listener),
        Err(ListenError::InvalidIp(_) | ListenError::InvalidPort(_)) => {
            eprintln!("bad input: {input}")
        },
        Err(ListenError::SacredPort) => eprintln!("port 85 is sacred"),
        Err(ListenError::Bind(source)) => eprintln!("couldn't bind: {source}"),
    }
}
```

All the original errors are there and most of `listen` is just `?`. We didn't make one `InvalidInput` though, or keep the address and available ports for the caller.

The problem is, it looks so cool in the function, we get to exploit '?', the problem is it fucking sucks reading the error, and you've not designed anything you've just thrown the errors up one layer with a typed boilerplate thingy.

## thiserror, verbose style (2.0.20)

Now the error is shaped around the task, without dropping those parser errors.

```rust
#[derive(Debug, thiserror::Error)]
pub enum InvalidInputError {
    #[error(transparent)]
    Ip(#[from] AddrParseError),
    #[error(transparent)]
    Port(#[from] ParseIntError),
}

// Making it
#[derive(Debug, thiserror::Error)]
pub enum ListenError {
    #[error("bad input: {input}")]
    InvalidInput {
        input: String,
        #[source]
        source: InvalidInputError,
    },
    #[error("port 85 is sacred")]
    SacredPort,
    #[error("couldn't bind {address}: {kind:?}")]
    BindFailed {
        address: SocketAddrV4,
        kind: io::ErrorKind,
        available_ports: Vec<u16>,
        #[source]
        source: io::Error,
    },
}

pub fn listen(input: &str) -> Result<TcpListener, ListenError> {
    // Normal stuff
    let (ip, port) = input.split_once(':').unwrap_or((input, ""));

    // First 2 errors, to us they're both just bad input
    let ip = ip
        .parse::<Ipv4Addr>()
        .map_err(|source| ListenError::InvalidInput {
            input: input.to_owned(),
            source: source.into(),
        })?;
    let port = port
        .parse::<u16>()
        .map_err(|source| ListenError::InvalidInput {
            input: input.to_owned(),
            source: source.into(),
        })?;

    // Third error, our own rule that port 85 is sacred
    if port == 85 {
        return Err(ListenError::SacredPort);
    }

    // Fourth error, we might want to match on what happened
    let address = SocketAddrV4::new(ip, port);

    TcpListener::bind(address).map_err(|source| {
        let kind = source.kind();
        let available_ports = find_available_ports(address);
        ListenError::BindFailed { address, kind, available_ports, source }
    })
}

// Using it
fn main() {
    let input = "127.0.0.1:8080";
    match listen(input) {
        Ok(listener) => start_server(listener),
        Err(error) => {
            match &error {
                ListenError::InvalidInput { input, .. } => eprintln!("bad input: {input}"),
                ListenError::SacredPort => eprintln!("port 85 is sacred"),
                ListenError::BindFailed { address, kind: io::ErrorKind::AddrInUse, available_ports, .. } => {
                    eprintln!("{address} is already in use");
                    show_available_ports(available_ports.as_slice());
                },
                ListenError::BindFailed { address, .. } => eprintln!("couldn't bind {address}"),
            }
        }
    }
}
```

Now both parser errors become `InvalidInput` and stay underneath it. That's what the task asked for, it just took another enum and all the manual mapping.

Is this bad? No ofcourse not, but i'll bet you that you are used and biased... think about it more. The amount of code exploded, if you saw this and didn't know it, you would probably think this was crazy.

As a reminder this:
```rust
  let ip = ip.parse::<Ipv4Addr>()?;
```
Turned into this:
```rust
  let ip = ip
      .parse::<Ipv4Addr>()
      .map_err(|source| ListenError::InvalidInput {
          input: input.to_owned(),
          source: source.into(),
      })?;
```

## Anyhow (1.0.104)

```rust
use anyhow::{Context, bail};

fn listen(input: &str) -> anyhow::Result<TcpListener> {
    // Normal stuff
    let (ip, port) = input.split_once(':').unwrap_or((input, ""));

    // First 2 errors, same context is enough here
    let ip = ip.parse::<Ipv4Addr>()
        .with_context(|| format!("bad input: {input}"))?;
    let port = port.parse::<u16>()
        .with_context(|| format!("bad input: {input}"))?;

    // Third error, our own rule that port 85 is sacred
    if port == 85 {
        bail!("port 85 is sacred");
    }

    // Fourth error, put what we found in the report
    let address = SocketAddrV4::new(ip, port);
    TcpListener::bind(address).with_context(|| {
        let available_ports = find_available_ports(address);
        format!("couldn't bind {address}, available ports: {available_ports:?}")
    })
}

// Using it
fn main() {
    match listen("127.0.0.1:8080") {
        Ok(listener) => start_server(listener),
        Err(error) => eprintln!("{error:?}"),
    }
}
```

This is the Anyhow ish style many people do, and i would even say many people don't even add context but just do the old classic '?' and get on with their day. The io error is still there though so we can downcast and read its `kind()`. 

Available ports are in the message not a `Vec<u16>` the caller can use, if we wanna get the typed experience some people combine it with thiserror afaik, but i mean then whats the point. 

## SNAFU (0.9.2)

```rust
use snafu::ResultExt;

// Making it
#[derive(Debug, snafu::Snafu)]
pub enum ListenError {
    #[snafu(display("bad input: {input}"))]
    InvalidInput { input: String, source: Box<dyn std::error::Error + Send + Sync> },
    #[snafu(display("port 85 is sacred"))]
    SacredPort,
    #[snafu(display("couldn't bind {address}: {source}"))]
    BindFailed {
        address: SocketAddrV4,
        kind: io::ErrorKind,
        available_ports: Vec<u16>,
        source: io::Error,
    },
}

pub fn listen(input: &str) -> Result<TcpListener, ListenError> {
    // Normal stuff
    let (ip, port) = input.split_once(':').unwrap_or((input, ""));

    // First 2 errors, to us they're both just bad input
    let ip = ip.parse::<Ipv4Addr>()
        .boxed()
        .context(InvalidInputSnafu { input })?;
    let port = port.parse::<u16>()
        .boxed()
        .context(InvalidInputSnafu { input })?;

    // Third error, our own rule that port 85 is sacred
    if port == 85 {
        return Err(ListenError::SacredPort);
    }

    // Fourth error, we might want to match on what happened
    let address = SocketAddrV4::new(ip, port);

    TcpListener::bind(address).with_context(|source| {
        let available_ports = find_available_ports(address);
        BindFailedSnafu { address, kind: source.kind(), available_ports }
    })
}

// Using it
fn main() {
    let input = "127.0.0.1:8080";
    match listen(input) {
        Ok(listener) => start_server(listener),
        Err(error) => {
            match &error {
                ListenError::InvalidInput { input, .. } => eprintln!("bad input: {input}"),
                ListenError::SacredPort => eprintln!("port 85 is sacred"),
                ListenError::BindFailed { address, kind: io::ErrorKind::AddrInUse, available_ports, .. } => {
                    eprintln!("{address} is already in use");
                    show_available_ports(available_ports.as_slice());
                },
                ListenError::BindFailed { address, .. } => eprintln!("couldn't bind {address}"),
            }
        }
    }
}
```

I think snafu is just anyhow++, that ain't bad, its ok.

## error-stack (0.8.0) (+ thiserror)

```rust
use error_stack::{Report, ResultExt};

// Making it
#[derive(Debug, thiserror::Error)]
pub enum ListenError {
    #[error("bad input: {input}")]
    InvalidInput { input: String },
    #[error("port 85 is sacred")]
    SacredPort,
    #[error("couldn't bind {address}")]
    BindFailed { address: SocketAddrV4, kind: io::ErrorKind, available_ports: Vec<u16> },
}

fn listen(input: &str) -> Result<TcpListener, Report<ListenError>> {
    // Normal stuff
    let (ip, port) = input.split_once(':').unwrap_or((input, ""));

    // First 2 errors, to us they're both just bad input
    let ip = ip.parse::<Ipv4Addr>()
        .change_context_lazy(|| ListenError::InvalidInput { input: input.to_owned() })?;
    let port = port.parse::<u16>()
        .change_context_lazy(|| ListenError::InvalidInput { input: input.to_owned() })?;

    // Third error, our own rule that port 85 is sacred
    if port == 85 {
        return Err(Report::new(ListenError::SacredPort));
    }

    // Fourth error, we might want to match on what happened
    let address = SocketAddrV4::new(ip, port);

    // change_context_lazy can't read kind() for us, so we map this bind.
    TcpListener::bind(address).map_err(|source| {
        let kind = source.kind();
        let available_ports = find_available_ports(address);
        Report::new(source).change_context(ListenError::BindFailed {
            address,
            kind,
            available_ports,
        })
    })
}

// Using it
fn main() {
    match listen("127.0.0.1:8080") {
        Ok(listener) => start_server(listener),
        Err(error) => {
            match error.current_context() {
                ListenError::InvalidInput { input } => eprintln!("bad input: {input}"),
                ListenError::SacredPort => eprintln!("port 85 is sacred"),
                ListenError::BindFailed { address, kind: io::ErrorKind::AddrInUse, available_ports } => {
                    eprintln!("{address} is already in use");
                    show_available_ports(available_ports.as_slice());
                },
                ListenError::BindFailed { address, .. } => eprintln!("couldn't bind {address}"),
            }
        }
    }
}
```

I feel like its exn but more verbose.

## rootcause (0.13.0) (+ thiserror)

```rust
use rootcause::{prelude::*, Report};

// Making it
#[derive(Debug, thiserror::Error)]
pub enum ListenError {
    #[error("bad input: {input}")]
    InvalidInput { input: String },
    #[error("port 85 is sacred")]
    SacredPort,
    #[error("couldn't bind {address}")]
    BindFailed { address: SocketAddrV4, kind: io::ErrorKind, available_ports: Vec<u16> },
}

fn listen(input: &str) -> Result<TcpListener, Report<ListenError>> {
    // Normal stuff
    let (ip, port) = input.split_once(':').unwrap_or((input, ""));

    // First 2 errors, to us they're both just bad input
    let ip = ip.parse::<Ipv4Addr>()
        .context_with(|| ListenError::InvalidInput { input: input.to_owned() })?;
    let port = port.parse::<u16>()
        .context_with(|| ListenError::InvalidInput { input: input.to_owned() })?;

    // Third error, our own rule that port 85 is sacred
    if port == 85 {
        return Err(rootcause::report!(ListenError::SacredPort));
    }

    // Fourth error, we might want to match on what happened
    let address = SocketAddrV4::new(ip, port);

    // We CANNOT use `context_transform`, it can read the failed io error, but it replaces it. So we get the kind but lose the original io error. Adding context keeps it, but that callback can't read it.
    // So gotta use map_err
    TcpListener::bind(address).map_err(|source| {
        let kind = source.kind();
        let available_ports = find_available_ports(address);
        Report::new(source).context(ListenError::BindFailed {
            address,
            kind,
            available_ports,
        })
    })
}

// Using it
fn main() {
    match listen("127.0.0.1:8080") {
        Ok(listener) => start_server(listener),
        Err(error) => {
            match error.current_context() {
                ListenError::InvalidInput { input } => eprintln!("bad input: {input}"),
                ListenError::SacredPort => eprintln!("port 85 is sacred"),
                ListenError::BindFailed { address, kind: io::ErrorKind::AddrInUse, available_ports } => {
                    eprintln!("{address} is already in use");
                    show_available_ports(available_ports.as_slice());
                },
                ListenError::BindFailed { address, .. } => eprintln!("couldn't bind {address}"),
            }
        }
    }
}
```

Similar to error-stack.

## exn (0.3.1) (+ thiserror)

```rust
use exn::{Exn, ResultExt};

// Making it
#[derive(Debug, thiserror::Error)]
pub enum ListenError {
    #[error("bad input: {input}")]
    InvalidInput { input: String },
    #[error("port 85 is sacred")]
    SacredPort,
    #[error("couldn't bind {address}")]
    BindFailed { address: SocketAddrV4, kind: io::ErrorKind, available_ports: Vec<u16> },
}

fn listen(input: &str) -> exn::Result<TcpListener, ListenError> {
    // Normal stuff
    let (ip, port) = input.split_once(':').unwrap_or((input, ""));

    // First 2 errors, to us they're both just bad input
    let ip = ip.parse::<Ipv4Addr>()
        .or_raise(|| ListenError::InvalidInput { input: input.to_owned() })?;
    let port = port.parse::<u16>()
        .or_raise(|| ListenError::InvalidInput { input: input.to_owned() })?;

    // Third error, our own rule that port 85 is sacred
    if port == 85 {
        return Err(Exn::new(ListenError::SacredPort));
    }

    // Fourth error, we might want to match on what happened
    let address = SocketAddrV4::new(ip, port);

    // or_raise doesn't get the io error, so we read its kind first.
    TcpListener::bind(address).map_err(|source| {
        let kind = source.kind();
        let available_ports = find_available_ports(address);
        Exn::new(source).raise(ListenError::BindFailed {
            address,
            kind,
            available_ports,
        })
    })
}

// Using it
fn main() {
    match listen("127.0.0.1:8080") {
        Ok(listener) => start_server(listener),
        Err(error) => {
            match &*error {
                ListenError::InvalidInput { input } => eprintln!("bad input: {input}"),
                ListenError::SacredPort => eprintln!("port 85 is sacred"),
                ListenError::BindFailed { address, kind: io::ErrorKind::AddrInUse, available_ports } => {
                    eprintln!("{address} is already in use");
                    show_available_ports(available_ports.as_slice());
                },
                ListenError::BindFailed { address, .. } => eprintln!("couldn't bind {address}"),
            }
        }
    }
}
```

Similar to error-stack and rootcause.

## Er (0.2.0)

```rust
use er::*;

// Making it
#[derive(Er)]
pub enum ListenErr {
    InvalidInput { input: String },
    SacredPort,
    BindFailed { address: SocketAddrV4, kind: io::ErrorKind, available_ports: Vec<u16> },
}
pub fn listen(input: &str) -> Er<TcpListener, ListenErr> {
    // Normal stuff
    let (ip, port) = input.split_once(':').unwrap_or((input, ""));

    // First 2 errors, to us they're both just bad input
    let ip = ip.parse::<Ipv4Addr>().er(|| ListenErr::invalid_input(input))?;
    let port = port.parse::<u16>().er(|| ListenErr::invalid_input(input))?;
    // If it was a struct error, it would be even shorter with like:
    // `let ip = ip.parse::<Ipv4Addr>().er(|_| input)?;`

    // Third error, our own rule that port 85 is sacred
    if port == 85 {
        return Err(ListenErr::sacred_port().er());
    }

    // Fourth error, we might want to match on what happened
    let address = SocketAddrV4::new(ip, port);
    TcpListener::bind(address).er_with(|e| {
        let available_ports = find_available_ports(address);
        ListenErr::bind_failed(address, e.kind(), available_ports)
    })
}

// Using it ourselves (still with Er)
fn main() {
    let input = "127.0.0.1:8080";
    match listen(input) {
        Ok(listener) => start_server(listener),
        Err(error) => {
            match &error.top {
                ListenErr::InvalidInput { input } => eprintln!("bad input: {input}"),
                ListenErr::SacredPort => eprintln!("port 85 is sacred"),
                ListenErr::BindFailed { address, kind: io::ErrorKind::AddrInUse, available_ports } => {
                    eprintln!("{address} is already in use");
                    show_available_ports(available_ports.as_slice());
                },
                ListenErr::BindFailed { address, .. } => eprintln!("couldn't bind {address}"),
            }
        }
    }
}
```

## Eros (0.8.0-rc.1) (+ thiserror)

Eros keeps the errors in a union and puts the extra information in context.

```rust
use eros::{Context, E4, ErrorUnion, ReshapeUnion};

// Making it
#[derive(Debug, thiserror::Error)]
#[error("port 85 is sacred")]
pub struct SacredPortError;

#[derive(Debug, thiserror::Error)]
#[error("couldn't bind {address}, available ports: {available_ports:?}")]
pub struct BindContext {
    pub address: SocketAddrV4,
    pub available_ports: Vec<u16>,
}

pub type ListenError = (AddrParseError, ParseIntError, SacredPortError, io::Error);

pub fn listen(input: &str) -> eros::Result<TcpListener, ListenError> {
    // Normal stuff
    let (ip, port) = input.split_once(':').unwrap_or((input, ""));

    // First 2 errors, same message but still separate errors in the union
    let ip = ip.parse::<Ipv4Addr>()
        .with_context(|| format!("bad input: {input}"))
        .widen()?;
    let port = port.parse::<u16>()
        .with_context(|| format!("bad input: {input}"))
        .widen()?;

    // Third error, our own rule that port 85 is sacred
    if port == 85 {
        return Err(ErrorUnion::new(SacredPortError));
    }

    // Fourth error, we might want to match on what happened
    let address = SocketAddrV4::new(ip, port);
    TcpListener::bind(address)
        .with_context(|| {
            let available_ports = find_available_ports(address);
            Box::new(BindContext { address, available_ports }) as Box<dyn eros::SendSyncError>
        })
        .widen()
}

// Using the union
fn main_with_union() {
    let input = "127.0.0.1:8080";
    match listen(input) {
        Ok(listener) => start_server(listener),
        Err(error) => {
            // E4 is the enum Eros makes from the tuple above.
            match error.as_enum() {
                // We already have input here. It isn't a field on the error.
                E4::A(_) | E4::B(_) => eprintln!("bad input: {input}"),
                E4::C(_) => eprintln!("port 85 is sacred"),
                E4::D(source) => {
                    // The io error is typed, but its context needs a runtime check.
                    let context = error.latest_context_error()
                        .and_then(|e| e.as_any().downcast_ref::<BindContext>());
                    if let Some(context) = context {
                        if source.kind() == io::ErrorKind::AddrInUse {
                            eprintln!("{} is already in use", context.address);
                            show_available_ports(context.available_ports.as_slice());
                        } else {
                            eprintln!("couldn't bind {}", context.address);
                        }
                    } else {
                        eprintln!("{error}");
                    }
                },
            }
        }
    }
}
```

Both parses get the same message, but they're still two types in the union. The ports are in `BindContext`, which the caller has to look up. The kind is on the io error. This version didn't give us one `InvalidInput` or a bind case with both pieces of data. Eros can turn these into our own errors too, but then we have to make those types and map into them.

I think it feels great in `listen()` but not so much in consuming it.

## Problemo (0.0.13)

Problemo's [guide](https://docs.rs/crate/problemo/0.0.13/source/README.md) says small tags for what happened and attachments for the data.

```rust
use problemo::*;

// Making it
tag_error!(InvalidInputError, "bad input");
tag_error!(SacredPortError, "port 85 is sacred");
tag_error!(BindFailedError, "couldn't bind");

pub struct Input(pub String);

pub struct BindDetails {
    pub address: SocketAddrV4,
    pub kind: io::ErrorKind,
    pub available_ports: Vec<u16>,
}

fn listen(input: &str) -> Result<TcpListener, Problem> {
    // Normal stuff
    let (ip, port) = input.split_once(':').unwrap_or((input, ""));

    // First 2 errors, to us they're both just bad input
    let ip = ip.parse::<Ipv4Addr>()
        .via(InvalidInputError)
        .map_with(|| Input(input.to_owned()))?;
    let port = port.parse::<u16>()
        .via(InvalidInputError)
        .map_with(|| Input(input.to_owned()))?;

    // Third error, our own rule that port 85 is sacred
    if port == 85 {
        return Err(SacredPortError::as_problem());
    }

    // Fourth error, we might want to match on what happened
    let address = SocketAddrV4::new(ip, port);

    // map_with doesn't get the io error, and we want its kind in the attachment.
    TcpListener::bind(address).map_err(|source| {
        let kind = source.kind();
        let available_ports = find_available_ports(address);
        source.into_problem()
            .via(BindFailedError)
            .with(BindDetails { address, kind, available_ports })
    })
}

// Using it
fn main() {
    match listen("127.0.0.1:8080") {
        Ok(listener) => start_server(listener),
        Err(problem) => {
            // Problem doesn't tell Rust which tags and attachments are in there.
            if problem.has_error_type::<InvalidInputError>() {
                if let Some(input) = problem.attachment_of_type::<Input>() {
                    eprintln!("bad input: {}", input.0);
                } else {
                    eprintln!("{problem}");
                }
            } else if problem.has_error_type::<SacredPortError>() {
                eprintln!("port 85 is sacred");
            } else if problem.has_error_type::<BindFailedError>() {
                if let Some(details) = problem.attachment_of_type::<BindDetails>() {
                    if details.kind == io::ErrorKind::AddrInUse {
                        eprintln!("{} is already in use", details.address);
                        show_available_ports(details.available_ports.as_slice());
                    } else {
                        eprintln!("couldn't bind {}", details.address);
                    }
                } else {
                    eprintln!("{problem}");
                }
            } else {
                eprintln!("{problem}");
            }
        },
    }
}
```

Both parses get the same bad input tag. Finding the bind tag still doesn't tell us `BindDetails` is attached, so the caller has to check for that.

# Getting the original errors

If we get `DriverError`, with an io error inside it, and we add context can we still get both actual errors back?

```rust
use std::{error::Error, fmt, io};

#[derive(Debug)]
pub struct DriverError {
    source: io::Error,
}
impl fmt::Display for DriverError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("device failed")
    }
}
impl Error for DriverError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.source)
    }
}

fn read_device() -> Result<(), DriverError> {
    Err(DriverError {
        source: io::Error::new(io::ErrorKind::PermissionDenied, "no access"),
    })
}
```

`DriverError.source()` points at the io error(but it returns `dyn Error` so we still have to check the type).

## Er

```rust
use er::*;

#[derive(Er)]
pub struct DeviceErr;
fn inspect_device() {
    let Err(error) = read_device().er::<DeviceErr>(()) else { return; };

    let original = error.er_find::<DriverError>(); // Found in the tree
    let nested = error.er_find::<io::Error>(); // Found inside DriverError too
}
```

Both are runtime lookups. `.er_find()` searches the tree and the native sources for us.

## Eros

```rust
use eros::Context;

fn inspect_device() {
    let result: eros::Result<(), (DriverError,)> = read_device().with_context(|| "read device");
    let Err(error) = result else { return; };

    let original: &DriverError = error.as_ref(); // Direct reference
    let nested = error.source()
        .and_then(|source| source.downcast_ref::<io::Error>()); // Found after checking the type
}
```

The union gives us `DriverError` directly. Its `source()` gives us the io error after a type check. We know it's one step down here. Eros isn't searching a whole chain for us.

## thiserror

```rust
#[derive(Debug, thiserror::Error)]
#[error("read device")]
pub struct DeviceError {
    pub source: DriverError,
}

fn inspect_device() {
    let Err(error) = read_device().map_err(|source| DeviceError { source }) else { return; };

    let original = &error.source; // Direct field
    let nested = original.source()
        .and_then(|source| source.downcast_ref::<io::Error>()); // Found after checking the type
}
```

The source field gives us `DriverError` directly. Its own `source()` returns `dyn Error`, so we check that it's an io error.

## SNAFU

```rust
use snafu::ResultExt;

#[derive(Debug, snafu::Snafu)]
#[snafu(display("read device"))]
pub struct DeviceError {
    pub source: DriverError,
}

fn inspect_device() {
    let Err(error) = read_device().context(DeviceSnafu) else { return; };

    let original = &error.source; // Direct field
    let nested = snafu::ChainCompat::new(&error)
        .find_map(|source| source.downcast_ref::<io::Error>()); // Found in the chain
}
```

The source field gives us `DriverError`. SNAFU's [chain iterator](https://docs.rs/snafu/0.9.2/snafu/struct.ChainCompat.html) walks its sources, and we check each one for `io::Error`.

## Anyhow

```rust
use anyhow::Context;

fn inspect_device() {
    let Err(error) = read_device().context("read device") else { return; };

    let original = error.downcast_ref::<DriverError>(); // Found by type
    let nested = error.chain()
        .find_map(|source| source.downcast_ref::<io::Error>()); // Found in the chain
}
```

`downcast_ref` finds `DriverError`. [`.chain()`](https://docs.rs/anyhow/1.0.104/anyhow/struct.Chain.html) walks its sources, so we can find the io error inside it too.

## error-stack

```rust
use error_stack::ResultExt;

#[derive(Debug, thiserror::Error)]
#[error("read device")]
pub struct DeviceError;

fn inspect_device() {
    let Err(error) = read_device().change_context(DeviceError) else { return; };

    let original = error.downcast_ref::<DriverError>(); // Found in the report
    let nested_in_frames = error.downcast_ref::<io::Error>(); // None
}
```

error-stack [copies the io error's message into a frame](https://docs.rs/error-stack/0.8.0/src/error_stack/context.rs.html). Its lookup finds `DriverError`, but that frame isn't an `io::Error`, so the io lookup fails. The real one is still inside `DriverError`. We'd have to follow its normal `source()` ourselves to get it.

## rootcause

```rust
use rootcause::prelude::*;

fn inspect_device() {
    let Err(error) = read_device().context("read device") else { return; };

    let original = error.iter_reports()
        .find_map(|report| report.downcast_current_context::<DriverError>()); // Found in the reports
    let nested = error.iter_reports()
        .filter_map(|report| report.current_context_error_source())
        .find_map(|source| source.downcast_ref::<io::Error>()); // Found in its source
}
```

We search the reports for `DriverError`. For the io error inside it, we ask each report for its [native source](https://docs.rs/rootcause/0.13.0/rootcause/struct.ReportRef.html#method.current_context_error_source) and check the type ourselves.

## exn

Exn keeps `DriverError` in a frame, but has no built in search. The [find_error people issues/request](https://github.com/fast/exn/issues/65) only searches frames. Exn [copies the io source's message into another frame as text](https://docs.rs/exn/0.3.1/exn/struct.Exn.html#method.new), so that search still wouldn't find the real `io::Error` inside `DriverError`. The real one is still there, but we'd have to follow `source()` manually.

## Problemo

```rust
use problemo::*;

tag_error!(DeviceError, "read device");

fn inspect_device() {
    let Err(error) = read_device().via(DeviceError) else { return; };

    let original = error.cause_with_error_type::<DriverError>(); // Found
    let nested = error.cause_with_error_type::<io::Error>(); // Found inside DriverError too
}
```

Problemo's [cause lookup](https://docs.rs/problemo/0.0.13/problemo/trait.Causes.html#method.cause_with_error_type) finds both. It already follows native sources, so we don't have to write that search ourselves.

# Public errors

Now someone else calls our code. Can we give them our error without making them use the same error library?

Some of these errors are already plain types. Some are inside a report or tree. For Eros, Anyhow and Problemo we'll make this one:

```rust
#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("bad input: {input}")]
    InvalidInput { input: String },
    #[error("port 85 is sacred")]
    SacredPort,
    #[error("couldn't bind {address}")]
    BindFailed { address: SocketAddrV4, available_ports: Vec<u16> },
    #[error("{0}")]
    Other(String),
}
```

I chose to give the public caller the address and ports, but not the io kind. `Other` is for when a lookup doesn't find our case or its data.

## Er

We already made `ListenErr` with the cases and fields we want to give them. We can hand out that type directly.

```rust
pub fn public_error_example(input: &str) -> Result<TcpListener, ListenErr> {
    listen(input).map_err(|error| error.top)
}
```

## Eros

Eros's [public API guide](https://docs.rs/eros/0.8.0-rc.1/eros/#public-apis) suggests returning the union too, especially between internal crates. Then the caller has to use Eros to handle it. Here we want to give them a plain error, so we convert it:

```rust
pub fn public_error_example(input: &str) -> Result<TcpListener, ApiError> {
    listen(input).map_err(|error| {
        let context = error.latest_context_error()
            .and_then(|e| e.as_any().downcast_ref::<BindContext>());

        match error.as_enum() {
            // We still have input as this function's argument, not from the union.
            E4::A(_) | E4::B(_) => ApiError::InvalidInput { input: input.to_owned() },
            E4::C(_) => ApiError::SacredPort,
            E4::D(source) => match context {
                Some(context) => ApiError::BindFailed {
                    address: context.address,
                    available_ports: context.available_ports.clone(),
                },
                None => ApiError::Other(source.to_string()),
            },
        }
    })
}
```

## thiserror, lazy style

`listen()` returns a normal enum, so the caller can match on it without using thiserror. Its cases are still the errors we got though, and the bind case doesn't have the address or ports we wanted to give them.

## thiserror, verbose style

This `ListenError` already has the cases and fields we chose for the caller. They can match on it without using thiserror.

## Anyhow

The context is text now, so there aren't typed stuff to get out, can still give the caller a plain error.

```rust
pub fn public_error_example(input: &str) -> Result<TcpListener, ApiError> {
    listen(input).map_err(|error| ApiError::Other(format!("{error:#}")))
}
```

The parser and io errors are still in there and can be downcast, but sacred port and the available ports only exist in our messages. If the public caller needs those as real variants/data, we gotta make that typed before throwing it into Anyhow.

## SNAFU

`listen()` already returns our `ListenError`, so the caller can match on it without using SNAFU. The parser errors are behind the boxed source, if they care which one failed they gotta downcast it.

## error-stack

Our context already has the fields we want. For this public version, change the earlier derive to `#[derive(Debug, Clone, thiserror::Error)]`, then clone the error out of the report.

```rust
pub fn public_error_example(input: &str) -> Result<TcpListener, ListenError> {
    listen(input).map_err(|report| report.current_context().clone())
}
```

## rootcause

Our context already has the fields we want. Rootcause lets us move it out.

```rust
pub fn public_error_example(input: &str) -> Result<TcpListener, ListenError> {
    listen(input).map_err(|report| report.into_current_context())
}
```

## exn

Like error-stack, change the earlier derive to `#[derive(Debug, Clone, thiserror::Error)]` for this public version, then clone it out.

```rust
pub fn public_error_example(input: &str) -> Result<TcpListener, ListenError> {
    listen(input).map_err(|error| (*error).clone())
}
```

## Problemo

We look up the tags and attachments to make `ApiError`.

```rust
pub fn public_error_example(input: &str) -> Result<TcpListener, ApiError> {
    listen(input).map_err(|problem| {
        if problem.has_error_type::<InvalidInputError>() {
            if let Some(input) = problem.attachment_of_type::<Input>() {
                return ApiError::InvalidInput { input: input.0.clone() };
            }
        } else if problem.has_error_type::<SacredPortError>() {
            return ApiError::SacredPort;
        } else if problem.has_error_type::<BindFailedError>() {
            if let Some(details) = problem.attachment_of_type::<BindDetails>() {
                return ApiError::BindFailed {
                    address: details.address,
                    available_ports: details.available_ports.clone(),
                };
            }
        }
        ApiError::Other(problem.to_string())
    })
}
```

# Conclusion

I don't know man I'm so biased, i just think people are too stubborn, use anything you like, just be more open minded and honest, they all have flaws in their own ways. I just want to type less on my little keyboard so 70% of my code isnt some fancy error handling api some guy invented who doesn't even use it (or if he does, then how much copium is he on, has to be atleast as much as me). 

I can respect the people who just want to do '?' and be lazy, but i have a hard time seeing the point of people wanting giant verbose apis adding context, and i think if you give exn/er a try you'll like it. Things are always scary/weird until you give them a shot.

If you open the error libraries docs and they don't mention ANY flaws about their own thing (trying to make it sound completely jacked and perfect, sales person style), or if they don't compare themselves to other libraries atleast... I think that that's a bad sign. You as a user aren't picking between no libraries and an ideal case with no flaws, you are picking 1 error handling philosophy with their conveniences, but with a bunch of drawbacks as well, there are no perfect libraries they all suck in some way (yes also Er).

Thanks for reading, remember to like and subscribe or something, i like cats.
