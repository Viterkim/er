# Tricky Error Comparison - Overview

Here's a tricky example where we take foreign errors, own errors, the original error, string context, typed context, public boundary stuff, and try to see what its like doing it, and what its like consuming it.

DISCLAIMER: this is a made up error case to try to hit the pain points of different error handling libraries (including Er), I've tried to hit complexity with composition while stile making it as readable as possible for the examples. Again I'm obviously biased, but i tried my best.

## Before we start

Read the [simple comparison](simple-error-comparison.md) first (basic usage, context, output).

Links on this page: [Er](#er-012-1), [Eros](#eros-070--thiserror-1), [thiserror](#thiserror-2020-1), [Anyhow](#anyhow-10104--thiserror-1), [SNAFU](#snafu-092--thiserror-1), [error-stack](#error-stack-080--thiserror-1), [rootcause](#rootcause-0130--thiserror-1),
[exn](#exn-031--thiserror-1), [Problemo](#problemo-0013--thiserror-1)

## Opinion

The hottest take from me is that each library (because of ergonomics and inner workings) ALWAYS forces you to do a thing in a certain way. The way `Er`, `eros` and `thiserror` (3 very different cases) each works makes you structure errors fundamentally different.

In some cases you COULD make them more similar, but you're fighting an uphill battle. Er/Exn makes you always design your errors, Eros makes you narrow/widen what inner errors could occur, and thiserror makes you map the original error up.

My biggest point is, why is the most inner error sacred/something your caller above you should ever handle/know? As long as you have the original message, i think that's exactly what you need, and if original context is there with something like an error codes, that should be brought up into your own type.

If your caller needs to do stuff with variants, your caller should not get a pyramid of nested chained types, they should be getting 1 type of what happened they can match on. And if they don't care about what happened they should just be forced to (at a type level) to yeet it up one layer so the next layer also adds the call location(which is why i don't think you should have an AppErr in Er, so you can just '?' it at every step). 

## The quest / task we're about to do

We will try to listen, and here we'll just have:  `127.0.0.1:8080`.

A bad `Ip` or `port` we'll group to bad input. (So 2 errors -> 1 case, with string context)

I decided that port `85` is sacred. (One own made case, no original error)

If bind fails, we want to 'get some inner stuff'. So lets try to get the address, available ports, `BindAttemptErr`, and the original `io::Error`. (So typed context, our own error type, and the inner original error type).

Then we want to expose it, to someone at a public level who should NOT use our library.

(`BindAttemptErr` is only here because we're testing getting back TWO errors! Our own one and the original `io::Error`. `bind_listener()` makes it, then `listen()` puts `BindFailed` on top.)

Done on Rust 1.98.1

## TLDR (libraries compared, pain points, experience)

### Er (0.1.2)

`ListenEr` is the type the caller gets, so matching our 3 cases is easy. If they want `BindAttemptErr` or the raw `io::Error` though, those are below it in the tree. They have to ask `.er_find()` for one at runtime.

One thing Er is bad at: say you have 8 foreign errors and want 8 variants, each holding the original error (classic thiserror). Er normally puts those originals below your type, in the tree. `.er_with()` can look at one, but it can't move it into the variant too unless it can be cloned. `.er_val()` CAN move it, so I'm not saying Er can't do it. But then it isn't also below your type, and derive won't make it an `Error::source()`. If you're used to having a `source` on every variant, Er is awkward here.

Er isn't the only one here btw. In these examples (error-stack, rootcause and Exn) also keep the original below the type you match on.

### Eros (0.7.0) (+ thiserror)

Eros makes it easy to collect the errors we got, so here the Ip and port errors stay as 2 members of the tuple. The caller gets 4 cases when we wanted 3 (i think the library is fighting us here for this case). 

We CAN map both into our own `InvalidInput` first, but then we have to make that type and decide what to do with the 2 original errors (lose them, box them, or make another enum). 

The string context is on the report, not the type the caller matches on. And for bind we had to make a seperate `BindFailed` ourselves once the address and ports needed to be typed fields.

Also i don't think its very pleasant for the caller / using it example (main).

### thiserror (2.0.20)

We start with the 3 cases we wanted. Both parses get mapped to `InvalidInput`, and the caller can just match on it. But here we throw away the original parse errors to do that (keeping both under 1 variant would need a box or another enum). 

`BindAttemptErr` and io error are typed fields, but that also makes `BindAttemptErr` part of the public type.

### Anyhow (1.0.104) (+ thiserror)

The `.with_context` part is nice. Both parse errors get the same "bad input" message, and we still have the original errors. But `listen()` returns `anyhow::Error`. Nothing in that type tells the caller about bad input, sacred port, or bind failure.

So if the caller wants to show available ports on the bind fail, it has to ask "hello do we have `BindFailed`?" at runtime. Then check for sacred port, and assume anything else was bad input. If we add another error later, that last bit will still compile and call it bad input. We do the same checks again just to give someone a plain `ListenErr`.

To be fair, we COULD make `InvalidInput` a typed context instead of using a string. Then we wouldn't have to guess that whatever is left is bad input. We'd still have to look for all 3 types at runtime in the caller.

### SNAFU (0.9.2) (+ thiserror)

SNAFU keeps the original error in a typed `source` field, which is great for getting the `io::Error` back from bind. But the Ip parser and port parser return different error types. So here we end up with `InvalidIp` and `InvalidPort` in `ListenFailure`, when from our caller's POV both mean "bad input".

We have to match those two together when using it, then merge them again when we make the public error. We COULD make one `InvalidInput` with a box or another enum for the two sources, but now we're writing an extra type just to say the one thing we wanted to say.

### error-stack (0.8.0) (+ thiserror)

This gives us the 3 cases. The caller matches on `current_context()` and gets the address and ports right from `BindFailed`. The original parser and io errors are still in the report too.

But `ListenErr` doesn't tell us what is below it. To get the original `BindAttemptErr` or `io::Error`, we have to ask the report to find one by type with `downcast_ref()`, and get an `Option` back. (Same tradeoff as Er with `.er_find()`).

### rootcause (0.13.0) (+ thiserror)

We also get our 3 cases with `current_context()`, so the caller can handle bad input, sacred port, and bind failure without guessing. And the original errors are kept below that.

If the caller wants the `io::Error` though, there isn't a `source` field on `ListenErr` to follow. Here we walk through the reports and check each one for the type we're looking for. That works, but the caller has to write the search itself.

### exn (0.3.1) (+ thiserror)

Exn gives us the 3 cases on top, nice. But when we want the raw `io::Error`, `main` has to know the order we built the tree in: first `BindAttemptErr`, then io. Put one more error between them and we'd have to change that code.

And there's a nasty case if we try to fix this with [find_error from this issue](https://github.com/fast/exn/issues/65). That function only looks in Exn's frames. Our example gives Exn `io::Error` directly, so it works here. But in the thiserror example, `BindAttemptErr` owns the io error as its normal `source()`. Give THAT to `Exn::new()` and Exn copies the io error's message into a frame. Now the report shows the io error, but `find_error::<io::Error>()` can't find it (that frame is just a string).

That's the bit i think Exn dun goofed on, when framed it makes the error's `source()` chain into strings, not the real errors. You can't cast those strings back to `io::Error`. To find the real one, you have to go back through the error Exn kept and follow its `source()` chain. Er doesn't make string frames for sources, and `.er_find()` follows the real ones.

### Problemo (0.0.13) (+ thiserror)

We put our `ListenErr` in the problem for every error path, but `listen()` returns `Problem`. The caller has no idea whether `ListenErr` is inside it. So to match our 3 cases, they have to look for it at runtime. And that lookup can return `None`.

For a plain public error we need an `Other` case, because a missing `ListenErr` isn't one of our 3 cases. So the public type ends up as `Known(ListenErr)` or `Other(String)`.

## Shared code

We use this function in all of them (no reason to copy pasta again and again).

```rust
fn show_available_ports(ports: &[u16]) {
    if ports.len() < 5 {
        eprintln!("available ports: {ports:?}");
    } else {
        eprintln!("{} ports available", ports.len());
    }
}
```

# Tricky Error Comparison - Full Examples

## Er (0.1.2)

```rust
// Making it
#[derive(Er)]
pub struct BindAttemptErr {
    pub address: SocketAddrV4,
    pub kind: io::ErrorKind,
}

fn bind_listener(address: SocketAddrV4) -> Er<TcpListener, BindAttemptErr> {
    TcpListener::bind(address).er_with(|e| BindAttemptErr {
        address,
        kind: e.kind(),
    })
}

#[derive(Er)]
pub enum ListenEr {
    InvalidInput { input: String },
    SacredPort,
    BindFailed { address: SocketAddrV4, kind: io::ErrorKind, available_ports: Vec<u16> },
}
pub fn listen(input: &str) -> Er<TcpListener, ListenEr> {
    // Normal stuff
    let (ip, port) = input.split_once(':').unwrap_or((input, ""));

    // First 2 errors, to us they're both just bad input
    // Shared dynamic context is enough, and the line number is here anyway
    let ip = ip.parse::<Ipv4Addr>().er(|| ListenEr::invalid_input(input))?;
    let port = port.parse::<u16>().er(|| ListenEr::invalid_input(input))?;

    // Third error, our own rule that port 85 is sacred
    if port == 85 {
        return Err(ListenEr::sacred_port().er());
    }

    // Fourth error, we might want to match on what happened
    let address = SocketAddrV4::new(ip, port);
    // e.top below is BindAttemptErr from bind_listener
    bind_listener(address).er_with(|e| {
        ListenEr::bind_failed(address, e.top.kind, find_available_ports(address))
    })
}

// Provide public error
pub fn public_error_example(input: &str) -> Result<TcpListener, ListenEr> {
    listen(input).map_err(|error| error.top)
}

// Using it
fn main() {
    let input = "127.0.0.1:8080";
    match listen(input) {
        Ok(listener) => start_server(listener),
        Err(error) => {
            match &error.top {
                ListenEr::InvalidInput { input } => eprintln!("bad input: {input}"),
                ListenEr::SacredPort => eprintln!("port 85 is sacred"),
                ListenEr::BindFailed { address, kind: io::ErrorKind::AddrInUse, available_ports } => {
                    eprintln!("{address} is already in use");
                    show_available_ports(available_ports.as_slice());
                },
                ListenEr::BindFailed { address, .. } => eprintln!("couldn't bind {address}"),
            }
            // Below .top we have to search for the types.
            if let Some(bind) = error.er_find::<BindAttemptErr>() {
                eprintln!("bind attempt: {}", bind.address);
            }
            if let Some(source) = error.er_find::<io::Error>() {
                eprintln!("raw error code: {:?}", source.raw_os_error());
            }
        }
    }
}
```

`.top` is the `ListenEr` we match on. The original `BindAttemptErr` and `io::Error` are below it.

## Eros (0.7.0) (+ thiserror)

```rust
// Making it
#[derive(Debug, thiserror::Error)] // thiserror
#[error("port 85 is sacred")]
pub struct SacredPortError;

#[derive(Debug, thiserror::Error)] // thiserror
#[error("couldn't bind {address}: {source}")]
pub struct BindAttemptErr {
    pub address: SocketAddrV4,
    pub source: io::Error,
}

#[derive(Debug, thiserror::Error)] // thiserror
#[error("couldn't bind {address}: {source}")]
pub struct BindFailed {
    pub address: SocketAddrV4,
    pub available_ports: Vec<u16>,
    #[source]
    pub source: BindAttemptErr,
}

pub type ListenErrors = (AddrParseError, ParseIntError, SacredPortError, BindFailed);

fn listen(input: &str) -> eros::Result<TcpListener, ListenErrors> {
    // Normal stuff
    let (ip, port) = input.split_once(':').unwrap_or((input, ""));

    // First 2 errors, to us they're both just bad input
    let ip = ip.parse::<Ipv4Addr>()
        .with_context(|| format!("bad input: {input}"))
        .widen()?;
    let port = port.parse::<u16>()
        .with_context(|| format!("bad input: {input}"))
        .widen()?;

    // Third error, our own rule that port 85 is sacred
    if port == 85 {
        return Err(eros::ErrorUnion::new(SacredPortError));
    }

    // Fourth error, we might want to match on what happened
    let address = SocketAddrV4::new(ip, port);

    // First idea: keep io::Error as the fourth case and add the ports as context:
    // TcpListener::bind(address)
    //     .with_context(|| {
    //         Box::new(AvailablePorts { ports: find_available_ports(address) }) as Box<dyn eros::SendSyncError>
    //     })
    //     .widen()
    //
    // With context enabled that prints the ports, but the caller still gets io::Error here.
    // They need the address and ports to make a public BindFailed.
    // We COULD put both in boxed context and downcast them later, but Eros lets you disable context.
    // So we make BindFailed ourselves.
    TcpListener::bind(address)
        .map_err(|source| BindFailed {
            address,
            available_ports: find_available_ports(address),
            source: BindAttemptErr { address, source },
        })
        .into_union()
}

// Provide public error
#[derive(Debug, thiserror::Error)] // thiserror
pub enum ListenErr {
    #[error("bad input: {input}")]
    InvalidInput { input: String },
    #[error("port 85 is sacred")]
    SacredPort,
    #[error("couldn't bind {address}: {kind:?}")]
    BindFailed { address: SocketAddrV4, kind: io::ErrorKind, available_ports: Vec<u16> },
}

pub fn public_error_example(input: &str) -> Result<TcpListener, ListenErr> {
    listen(input).map_err(|error| match error.to_enum() {
        eros::E4::A(_) | eros::E4::B(_) => ListenErr::InvalidInput { input: input.to_owned() },
        eros::E4::C(_) => ListenErr::SacredPort,
        eros::E4::D(e) => ListenErr::BindFailed {
            address: e.address,
            kind: e.source.source.kind(),
            available_ports: e.available_ports,
        },
    })
}

// Using it
fn main() {
    let input = "127.0.0.1:8080";
    match listen(input) {
        Ok(listener) => start_server(listener),
        Err(error) => {
            match error.ref_enum() {
                eros::E4::A(_) | eros::E4::B(_) => eprintln!("bad input: {input}"),
                eros::E4::C(_) => eprintln!("port 85 is sacred"),
                eros::E4::D(e) if e.source.source.kind() == io::ErrorKind::AddrInUse => {
                    eprintln!("{} is already in use", e.address);
                    show_available_ports(e.available_ports.as_slice());
                },
                eros::E4::D(e) => eprintln!("couldn't bind {}", e.address),
            }
            if let eros::E4::D(bind) = error.ref_enum() {
                eprintln!("bind attempt: {}", bind.source.address);
                eprintln!("raw error code: {:?}", bind.source.source.raw_os_error());
            }
        }
    }
}
```

`BindFailed` has the address and ports. `BindAttemptErr` holds the original io error. To return a plain error without Eros, we map four union cases into three variants.

## thiserror (2.0.20)

```rust
// Making it
#[derive(Debug, thiserror::Error)] // thiserror
#[error("couldn't bind {address}: {source}")]
pub struct BindAttemptErr {
    pub address: SocketAddrV4,
    pub source: io::Error,
}

fn bind_listener(address: SocketAddrV4) -> Result<TcpListener, BindAttemptErr> {
    TcpListener::bind(address).map_err(|source| BindAttemptErr { address, source })
}

#[derive(Debug, thiserror::Error)] // thiserror
pub enum ListenErr {
    #[error("bad input: {input}")]
    InvalidInput { input: String },
    #[error("port 85 is sacred")]
    SacredPort,
    #[error("couldn't bind {address}: {kind:?}")]
    BindFailed {
        address: SocketAddrV4,
        kind: io::ErrorKind,
        available_ports: Vec<u16>,
        source: BindAttemptErr,
    },
}

pub fn listen(input: &str) -> Result<TcpListener, ListenErr> {
    // Normal stuff
    let (ip, port) = input.split_once(':').unwrap_or((input, ""));

    // First 2 errors, to us they're both just bad input
    let ip = ip.parse::<Ipv4Addr>()
        .map_err(|_| ListenErr::InvalidInput { input: input.to_owned() })?;
    let port = port.parse::<u16>()
        .map_err(|_| ListenErr::InvalidInput { input: input.to_owned() })?;

    // Third error, our own rule that port 85 is sacred
    if port == 85 {
        return Err(ListenErr::SacredPort);
    }

    // Fourth error, we might want to match on what happened
    let address = SocketAddrV4::new(ip, port);

    bind_listener(address).map_err(|source| ListenErr::BindFailed {
        address,
        kind: source.source.kind(),
        available_ports: find_available_ports(address),
        source,
    })
}

// Provide public error
// listen already returns Result<TcpListener, ListenErr>.

// Using it
fn main() {
    let input = "127.0.0.1:8080";
    match listen(input) {
        Ok(listener) => start_server(listener),
        Err(error) => {
            match &error {
                ListenErr::InvalidInput { input } => eprintln!("bad input: {input}"),
                ListenErr::SacredPort => eprintln!("port 85 is sacred"),
                ListenErr::BindFailed { address, kind: io::ErrorKind::AddrInUse, available_ports, .. } => {
                    eprintln!("{address} is already in use");
                    show_available_ports(available_ports.as_slice());
                },
                ListenErr::BindFailed { address, .. } => eprintln!("couldn't bind {address}"),
            }
            if let ListenErr::BindFailed { source, .. } = &error {
                eprintln!("bind attempt: {}", source.address);
                eprintln!("raw error code: {:?}", source.source.raw_os_error());
            }
        }
    }
}
```

The bind and io errors are typed fields here, no search. But now `BindAttemptErr` is part of the public type. The two parse errors are gone (to keep both in one `InvalidInput`, we'd need another source type or a box).

## Anyhow (1.0.104) (+ thiserror)

```rust
// Making it
#[derive(Debug, thiserror::Error)] // thiserror
#[error("couldn't bind {address}: {kind:?}")]
pub struct BindAttemptErr {
    pub address: SocketAddrV4,
    pub kind: io::ErrorKind,
}

#[derive(Debug, thiserror::Error)] // thiserror
#[error("port 85 is sacred")]
pub struct SacredPort;

#[derive(Debug, thiserror::Error)] // thiserror
#[error("couldn't bind {address}: {kind:?}")]
pub struct BindFailed {
    pub address: SocketAddrV4,
    pub kind: io::ErrorKind,
    pub available_ports: Vec<u16>,
}

fn listen(input: &str) -> anyhow::Result<TcpListener> {
    // Normal stuff
    let (ip, port) = input.split_once(':').unwrap_or((input, ""));

    // First 2 errors, to us they're both just bad input
    let ip = ip.parse::<Ipv4Addr>()
        .with_context(|| format!("bad input: {input}"))?;
    let port = port.parse::<u16>()
        .with_context(|| format!("bad input: {input}"))?;

    // Third error, our own rule that port 85 is sacred
    if port == 85 {
        return Err(SacredPort.into());
    }

    // Fourth error, we might want to match on what happened
    let address = SocketAddrV4::new(ip, port);

    TcpListener::bind(address).map_err(|source| {
        let kind = source.kind();
        anyhow::Error::new(source)
            .context(BindAttemptErr { address, kind })
            .context(BindFailed {
                address,
                kind,
                available_ports: find_available_ports(address),
            })
    })
}

// Provide public error
#[derive(Debug, thiserror::Error)] // thiserror
pub enum ListenErr {
    #[error("bad input: {input}")]
    InvalidInput { input: String },
    #[error("port 85 is sacred")]
    SacredPort,
    #[error("couldn't bind {address}: {kind:?}")]
    BindFailed { address: SocketAddrV4, kind: io::ErrorKind, available_ports: Vec<u16> },
}

pub fn public_error_example(input: &str) -> Result<TcpListener, ListenErr> {
    listen(input).map_err(|error| {
        // Compile time info lost, so have to do a runtime check for the type.
        if let Some(bind) = error.downcast_ref::<BindFailed>() {
            ListenErr::BindFailed {
                address: bind.address,
                kind: bind.kind,
                available_ports: bind.available_ports.clone(),
            }
        } else if error.is::<SacredPort>() {
            ListenErr::SacredPort
        } else {
            ListenErr::InvalidInput { input: input.to_owned() }
        }
    })
}

// Using it
fn main() {
    let input = "127.0.0.1:8080";
    match listen(input) {
        Ok(listener) => start_server(listener),
        Err(error) => {
            // Compile time info lost, so have to do a runtime check for the type.
            if let Some(bind) = error.downcast_ref::<BindFailed>() {
                if bind.kind == io::ErrorKind::AddrInUse {
                    eprintln!("{} is already in use", bind.address);
                    show_available_ports(bind.available_ports.as_slice());
                } else {
                    eprintln!("couldn't bind {}", bind.address);
                }
            } else if error.is::<SacredPort>() {
                eprintln!("port 85 is sacred");
            } else {
                eprintln!("bad input: {input}");
            }
            if let Some(bind) = error.downcast_ref::<BindAttemptErr>() {
                eprintln!("bind attempt: {}", bind.address);
            }
            if let Some(source) = error.downcast_ref::<io::Error>() {
                eprintln!("raw error code: {:?}", source.raw_os_error());
            }
        },
    }
}
```

The original errors are still there, but `anyhow::Error` doesn't say which one happened. Runtime type checks to use it, then the same checks again for the public error.

## SNAFU (0.9.2) (+ thiserror)

```rust
// Making it
#[derive(Debug, snafu::Snafu)]
#[snafu(display("couldn't bind {address}: {source}"))]
pub struct BindAttemptErr {
    pub address: SocketAddrV4,
    pub source: io::Error,
}

fn bind_listener(address: SocketAddrV4) -> Result<TcpListener, BindAttemptErr> {
    TcpListener::bind(address).context(BindAttemptErrSnafu { address })
}

#[derive(Debug, snafu::Snafu)]
pub enum ListenFailure {
    #[snafu(display("bad input: {input}"))]
    InvalidIp { input: String, source: AddrParseError },
    #[snafu(display("bad input: {input}"))]
    InvalidPort { input: String, source: ParseIntError },
    #[snafu(display("port 85 is sacred"))]
    SacredPort,
    #[snafu(display("couldn't bind {address}: {source}"))]
    BindFailed { address: SocketAddrV4, available_ports: Vec<u16>, source: BindAttemptErr },
}

fn listen(input: &str) -> Result<TcpListener, ListenFailure> {
    // Normal stuff
    let (ip, port) = input.split_once(':').unwrap_or((input, ""));

    // First 2 errors, to us they're both just bad input
    let ip = ip.parse::<Ipv4Addr>().context(InvalidIpSnafu { input })?;
    let port = port.parse::<u16>().context(InvalidPortSnafu { input })?;

    // Third error, our own rule that port 85 is sacred
    if port == 85 {
        return Err(ListenFailure::SacredPort);
    }

    // Fourth error, we might want to match on what happened
    let address = SocketAddrV4::new(ip, port);

    bind_listener(address).context(BindFailedSnafu {
        address,
        available_ports: find_available_ports(address),
    })
}

// Provide public error
#[derive(Debug, thiserror::Error)] // thiserror
pub enum ListenErr {
    #[error("bad input: {input}")]
    InvalidInput { input: String },
    #[error("port 85 is sacred")]
    SacredPort,
    #[error("couldn't bind {address}: {kind:?}")]
    BindFailed { address: SocketAddrV4, kind: io::ErrorKind, available_ports: Vec<u16> },
}

pub fn public_error_example(input: &str) -> Result<TcpListener, ListenErr> {
    listen(input).map_err(|error| match error {
        ListenFailure::InvalidIp { input, .. } | ListenFailure::InvalidPort { input, .. } => {
            ListenErr::InvalidInput { input }
        }
        ListenFailure::SacredPort => ListenErr::SacredPort,
        ListenFailure::BindFailed { address, available_ports, source } => ListenErr::BindFailed {
            address,
            kind: source.source.kind(),
            available_ports,
        },
    })
}

// Using it
fn main() {
    let input = "127.0.0.1:8080";
    match listen(input) {
        Ok(listener) => start_server(listener),
        Err(error) => {
            match &error {
                ListenFailure::InvalidIp { input, .. } | ListenFailure::InvalidPort { input, .. } => {
                    eprintln!("bad input: {input}");
                }
                ListenFailure::SacredPort => eprintln!("port 85 is sacred"),
                ListenFailure::BindFailed { address, available_ports, source }
                    if source.source.kind() == io::ErrorKind::AddrInUse => {
                        eprintln!("{address} is already in use");
                        show_available_ports(available_ports.as_slice());
                    },
                ListenFailure::BindFailed { address, .. } => eprintln!("couldn't bind {address}"),
            }
            if let ListenFailure::BindFailed { source, .. } = &error {
                eprintln!("bind attempt: {}", source.address);
                eprintln!("raw error code: {:?}", source.source.raw_os_error());
            }
        }
    }
}
```

SNAFU keeps the sources typed. Ip and port need separate variants because their source types differ (we merge them back into `InvalidInput` for the public error).

## error-stack (0.8.0) (+ thiserror)

```rust
// Making it
#[derive(Debug, thiserror::Error)] // thiserror
#[error("couldn't bind {address}: {kind:?}")]
pub struct BindAttemptErr {
    pub address: SocketAddrV4,
    pub kind: io::ErrorKind,
}

#[derive(Debug, Clone, thiserror::Error)] // thiserror
pub enum ListenErr {
    #[error("bad input: {input}")]
    InvalidInput { input: String },
    #[error("port 85 is sacred")]
    SacredPort,
    #[error("couldn't bind {address}: {kind:?}")]
    BindFailed { address: SocketAddrV4, kind: io::ErrorKind, available_ports: Vec<u16> },
}

fn listen(input: &str) -> Result<TcpListener, error_stack::Report<ListenErr>> {
    // Normal stuff
    let (ip, port) = input.split_once(':').unwrap_or((input, ""));

    // First 2 errors, to us they're both just bad input
    let ip = ip.parse::<Ipv4Addr>()
        .change_context_lazy(|| ListenErr::InvalidInput { input: input.to_owned() })?;
    let port = port.parse::<u16>()
        .change_context_lazy(|| ListenErr::InvalidInput { input: input.to_owned() })?;

    // Third error, our own rule that port 85 is sacred
    if port == 85 {
        return Err(error_stack::Report::new(ListenErr::SacredPort));
    }

    // Fourth error, we might want to match on what happened
    let address = SocketAddrV4::new(ip, port);

    TcpListener::bind(address).map_err(|source| {
        let kind = source.kind();
        error_stack::Report::new(source)
            .change_context(BindAttemptErr { address, kind })
            .change_context(ListenErr::BindFailed {
                address,
                kind,
                available_ports: find_available_ports(address),
            })
    })
}

// Provide public error
pub fn public_error_example(input: &str) -> Result<TcpListener, ListenErr> {
    listen(input).map_err(|report| report.current_context().clone())
}

// Using it
fn main() {
    match listen("127.0.0.1:8080") {
        Ok(listener) => start_server(listener),
        Err(error) => {
            // The other arms are the same as thiserror.
            if let ListenErr::BindFailed { kind: io::ErrorKind::AddrInUse, available_ports, .. } = error.current_context() {
                show_available_ports(available_ports.as_slice());
            }
            if let Some(bind) = error.downcast_ref::<BindAttemptErr>() {
                eprintln!("bind attempt: {}", bind.address);
            }
            if let Some(source) = error.downcast_ref::<io::Error>() {
                eprintln!("raw error code: {:?}", source.raw_os_error());
            }
        }
    }
}
```

We can match on `current_context()` directly. The bind attempt and io error need a lookup.

## rootcause (0.13.0) (+ thiserror)

```rust
// Making it
#[derive(Debug, thiserror::Error)] // thiserror
#[error("couldn't bind {address}: {kind:?}")]
pub struct BindAttemptErr {
    pub address: SocketAddrV4,
    pub kind: io::ErrorKind,
}

#[derive(Debug, Clone, thiserror::Error)] // thiserror
pub enum ListenErr {
    #[error("bad input: {input}")]
    InvalidInput { input: String },
    #[error("port 85 is sacred")]
    SacredPort,
    #[error("couldn't bind {address}: {kind:?}")]
    BindFailed { address: SocketAddrV4, kind: io::ErrorKind, available_ports: Vec<u16> },
}

fn listen(input: &str) -> Result<TcpListener, rootcause::Report<ListenErr>> {
    // Normal stuff
    let (ip, port) = input.split_once(':').unwrap_or((input, ""));

    // First 2 errors, to us they're both just bad input
    let ip = ip.parse::<Ipv4Addr>()
        .context_with(|| ListenErr::InvalidInput { input: input.to_owned() })?;
    let port = port.parse::<u16>()
        .context_with(|| ListenErr::InvalidInput { input: input.to_owned() })?;

    // Third error, our own rule that port 85 is sacred
    if port == 85 {
        return Err(rootcause::report!(ListenErr::SacredPort));
    }

    // Fourth error, we might want to match on what happened
    let address = SocketAddrV4::new(ip, port);

    TcpListener::bind(address).map_err(|source| {
        let kind = source.kind();
        rootcause::report!(source)
            .context(BindAttemptErr { address, kind })
            .context(ListenErr::BindFailed {
                address,
                kind,
                available_ports: find_available_ports(address),
            })
    })
}

// Provide public error
pub fn public_error_example(input: &str) -> Result<TcpListener, ListenErr> {
    listen(input).map_err(|report| report.into_current_context())
}

// Using it
fn main() {
    match listen("127.0.0.1:8080") {
        Ok(listener) => start_server(listener),
        Err(error) => {
            // The other arms are the same as thiserror.
            if let ListenErr::BindFailed { kind: io::ErrorKind::AddrInUse, available_ports, .. } = error.current_context() {
                show_available_ports(available_ports.as_slice());
            }
            if let Some(bind) = error.iter_reports()
                .find_map(|report| report.current_context_as_any().downcast_ref::<BindAttemptErr>()) {
                eprintln!("bind attempt: {}", bind.address);
            }
            if let Some(source) = error.iter_reports()
                .find_map(|report| report.current_context_as_any().downcast_ref::<io::Error>()) {
                eprintln!("raw error code: {:?}", source.raw_os_error());
            }
        }
    }
}
```

`into_current_context()` moves `ListenErr` out (no clone).

## exn (0.3.1) (+ thiserror)

```rust
// Making it
#[derive(Debug, thiserror::Error)] // thiserror
#[error("couldn't bind {address}: {kind:?}")]
pub struct BindAttemptErr {
    pub address: SocketAddrV4,
    pub kind: io::ErrorKind,
}

#[derive(Debug, Clone, thiserror::Error)] // thiserror
pub enum ListenErr {
    #[error("bad input: {input}")]
    InvalidInput { input: String },
    #[error("port 85 is sacred")]
    SacredPort,
    #[error("couldn't bind {address}: {kind:?}")]
    BindFailed { address: SocketAddrV4, kind: io::ErrorKind, available_ports: Vec<u16> },
}

fn listen(input: &str) -> exn::Result<TcpListener, ListenErr> {
    // Normal stuff
    let (ip, port) = input.split_once(':').unwrap_or((input, ""));

    // First 2 errors, to us they're both just bad input
    let ip = ip.parse::<Ipv4Addr>()
        .or_raise(|| ListenErr::InvalidInput { input: input.to_owned() })?;
    let port = port.parse::<u16>()
        .or_raise(|| ListenErr::InvalidInput { input: input.to_owned() })?;

    // Third error, our own rule that port 85 is sacred
    if port == 85 {
        return Err(exn::Exn::new(ListenErr::SacredPort));
    }

    // Fourth error, we might want to match on what happened
    let address = SocketAddrV4::new(ip, port);

    TcpListener::bind(address).map_err(|source| {
        let kind = source.kind();
        // Here we give Exn io::Error directly so it stays findable in the frames.
        exn::Exn::new(source)
            .raise(BindAttemptErr { address, kind })
            .raise(ListenErr::BindFailed {
                address,
                kind,
                available_ports: find_available_ports(address),
            })
    })
}

// Provide public error
pub fn public_error_example(input: &str) -> Result<TcpListener, ListenErr> {
    listen(input).map_err(|error| (*error).clone())
}

// Using it
fn main() {
    match listen("127.0.0.1:8080") {
        Ok(listener) => start_server(listener),
        Err(error) => {
            // The other arms are the same as thiserror.
            if let ListenErr::BindFailed { kind: io::ErrorKind::AddrInUse, available_ports, .. } = &*error {
                show_available_ports(available_ports.as_slice());
            }
            if let Some(frame) = error.frame().children().first() {
                if let Some(bind) = frame.error().downcast_ref::<BindAttemptErr>() {
                    eprintln!("bind attempt: {}", bind.address);
                }
                if let Some(source) = frame.children().first()
                    .and_then(|frame| frame.error().downcast_ref::<io::Error>()) {
                    eprintln!("raw error code: {:?}", source.raw_os_error());
                }
            }
        }
    }
}
```

`&*error` lets us match on `ListenErr`. To get the `io::Error`, we have to dig through the errors below it.

## Problemo (0.0.13) (+ thiserror)

```rust
// Making it
#[derive(Debug, thiserror::Error)] // thiserror
#[error("couldn't bind {address}: {kind:?}")]
pub struct BindAttemptErr {
    pub address: SocketAddrV4,
    pub kind: io::ErrorKind,
}

#[derive(Debug, Clone, thiserror::Error)] // thiserror
pub enum ListenErr {
    #[error("bad input: {input}")]
    InvalidInput { input: String },
    #[error("port 85 is sacred")]
    SacredPort,
    #[error("couldn't bind {address}: {kind:?}")]
    BindFailed { address: SocketAddrV4, kind: io::ErrorKind, available_ports: Vec<u16> },
}

fn listen(input: &str) -> Result<TcpListener, problemo::Problem> {
    // Normal stuff
    let (ip, port) = input.split_once(':').unwrap_or((input, ""));

    // First 2 errors, to us they're both just bad input
    let ip = ip.parse::<Ipv4Addr>()
        .map_via(|| ListenErr::InvalidInput { input: input.to_owned() })?;
    let port = port.parse::<u16>()
        .map_via(|| ListenErr::InvalidInput { input: input.to_owned() })?;

    // Third error, our own rule that port 85 is sacred
    if port == 85 {
        return Err(ListenErr::SacredPort.into());
    }

    // Fourth error, we might want to match on what happened
    let address = SocketAddrV4::new(ip, port);

    TcpListener::bind(address).map_err(|source| {
        let kind = source.kind();
        problemo::Problem::from(source)
            .via(BindAttemptErr { address, kind })
            .via(ListenErr::BindFailed {
                address,
                kind,
                available_ports: find_available_ports(address),
            })
    })
}

// Provide public error
#[derive(Debug, thiserror::Error)] // thiserror
pub enum PublicListenErr {
    #[error(transparent)]
    Known(#[from] ListenErr),
    #[error("{0}")]
    Other(String),
}

pub fn public_error_example(input: &str) -> Result<TcpListener, PublicListenErr> {
    listen(input).map_err(|problem| {
        // Compile time info lost, so have to do a runtime check for the type.
        match problem.cause_with_error_type::<ListenErr>() {
            Some(cause) => PublicListenErr::Known(cause.error.clone()),
            None => PublicListenErr::Other(problem.to_string()),
        }
    })
}

// Using it
fn main() {
    let input = "127.0.0.1:8080";
    match listen(input) {
        Ok(listener) => start_server(listener),
        Err(problem) => {
            // Compile time info lost, so have to do a runtime check for the type.
            if let Some(cause) = problem.cause_with_error_type::<ListenErr>() {
                // The other arms are the same as thiserror.
                if let ListenErr::BindFailed { kind: io::ErrorKind::AddrInUse, available_ports, .. } = cause.error {
                    show_available_ports(available_ports.as_slice());
                }
            } else {
                eprintln!("{problem}");
            }
            if let Some(bind) = problem.cause_with_error_type::<BindAttemptErr>() {
                eprintln!("bind attempt: {}", bind.error.address);
            }
            if let Some(source) = problem.cause_with_error_type::<io::Error>() {
                eprintln!("raw error code: {:?}", source.error.raw_os_error());
            }
        },
    }
}
```

`Problem` needs a runtime lookup for `ListenErr` and the two errors below it.
