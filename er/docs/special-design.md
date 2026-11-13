# Special design decisions in Er

## The library boundary

Say your library opens a port. When 85 is busy, the app wants to offer another port. When somebody sends you a bug report, you want to see what happened on their machine.

```rust
pub fn listen(port: u16) -> Result<TcpListener, (ListenErr, String)> {
    bind(port).er_split()
}
```

```rust
if let Err((ListenErr::Busy { available }, report)) = radio::listen(85) {
    eprintln!("{report}");
    show_available_ports(&available);
}
```

The app could have written that before Er existed. It gets port numbers it can use and a report it can paste into a bug. There's no Er type in that signature. The app can keep using its own version of Er, or just use the pair as it is.

You can see what that saves in libraries that exchange their own report types. `exn-anyhow` walks Anyhow's causes and builds a chain of Exn frames from the messages. Somebody had to write that conversion and decide to ship the chain as text. [exn-anyhow source](https://docs.rs/exn-anyhow/0.1.0/src/exn_anyhow/lib.rs.html#43-56)

It happens between versions too. Rootcause 0.13 has a Cargo feature and a conversion module for each of error-stack 0.5, 0.6 and 0.7. Rust sees three different `Report` types. With `(ListenErr, String)`, another error library already has the types it needs. [Rootcause's Cargo.toml](https://docs.rs/crate/rootcause/0.13.0/source/Cargo.toml.orig)

Give the app data it can use on `ListenErr`. The available ports are a `Vec<u16>` for a reason. If the app needs the `io::ErrorKind` to decide whether to try again, put that on `ListenErr` too. Once it's in the report string, the app only has words to work with.

## Make losing the report take some effort

Say your function returns `Result<(), AppErr>` and has a `From<ListenErr>` impl. `radio::listen(85)?` won't compile. The impl knows what to do with the error and there's still a report sitting next to it.

```rust
radio::listen(85).map_err(|(error, _)| error)?;
```

You can see where the report goes missing now. Never do this in Er code. In a function returning `ErResult<(), StartErr>`, `.er()` puts `ListenErr` under `StartErr` and keeps the report as saved text.

```rust
radio::listen(85).er(|| StartErr::listen(85))?;
```

Display and Debug have the same problem in a sneakier form. An error has no idea whether somebody is printing it by itself or a reporter is already about to print its cause. Put the cause in the message and a reporter that follows `source()` can print it twice. Leave it out and the log call that only prints the error gets a lonely line. A tree has neither Display nor Debug. At the place doing the printing you can see whether you wanted `error.top` for a dialog or `error.er_report()` for your logs.

## A little context before the question mark

```rust
#[derive(Er)]
pub enum CopyErr {
    Read(PathBuf),
    Write(PathBuf),
}

fn copy_file(input: &Path, output: &Path) -> ErResult<(), CopyErr> {
    let data = fs::read(input).er(|| CopyErr::read(input))?;

    fs::write(output, data).er(|| CopyErr::write(output))
}
```

Both I/O calls can give you `NotFound`. Was the input missing, or the directory you were copying into? A `From<io::Error>` impl sees the same error type and neither path. Right here we know which operation failed and which path it was using.

One layer up, `.er_with()` can turn a failed read into something the app knows how to handle. A match on `CopyErr` tells Rust which cases the app cares about. Add one and the compiler points to the match. A retry that searches for some inner `io::Error` can stop working as soon as somebody changes how the file is read.

I like thiserror for a public enum. Writing the outward message and wiring up a source for every private function feels like passing raw C pointers through your program because one function calls C. The copy function knows the paths. The app knows what to do when a copy fails. Let each one say what it knows.

## Cleanup can fail too

```rust
let work = server.run();
let closed = server.close();

er_all!(ShutdownErr, [work, closed])
```

A `?` on `run()` can skip cleanup. A `?` on `close()` can hide why the server stopped. These failures belong side by side under `ShutdownErr`. In the copy you need the bytes before you can write them, so there we use `?`.
