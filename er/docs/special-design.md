# Special design decisions in Er

A bunch of choices in Er look stupid at first, like returning a tuple `(MyErr, String)`, no `Debug` or `Display` on trees, putting `.er()` everywhere, making a tiny error type for almost every function...

However they all fit together and most of them exist because the obvious alternatives quietly suck or have very very bad behaviour when interacting with other designs. Many seemingly simple issues are quite tricky to find solutions for which fit everything else, and most things are because rusts Error trait is bad, and blanket implementation issues in rust.

## No `Debug` or `Display` on `ErTree`

In Rust nobody actually agrees on what `{error}` vs `{error:?}` should print. If you put the cause in `Display`, any reporter walking `source()` will print it twice. If you leave it out and log `{error}` you basically get 0 context.

So we exploit the type system by NOT implementing Debug/Display, so you are forced to decide and not use tribal knowledge tier level apis. Which is imo on the same level of doing javascript tier bullshit nontyped raw strings and giving them sentient values.

## Playing nice at the library boundary: `(MyErr, String)`

Er is for both libraries and applications, and someone using your library should never have to use Er (or the same version of Er as you).

Lets say you want the full report of what went wrong on their machine.

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

The consumer just gets normal Rust types which is a boring `ListenErr` with real data (`Vec<u16>`) to match on, and a `String` report they can log or paste into a bug report. They don't need Er, don't care what version of Er you're on, and don't even have to know Er exists.

It even bites you across versions of the same crate. Rootcause 0.13 has separate Cargo features and conversion modules for `error-stack` 0.5, 0.6 and 0.7 because Rust treats those as three different `Report` types. [Rootcause's Cargo.toml](https://docs.rs/crate/rootcause/0.13.0/source/Cargo.toml.orig)

And not directly related, but similar would be crates that hand their own report type across library boundaries. `exn-anyhow` has to walk Anyhow's causes and rebuild a chain of Exn frames out of text just to bridge two libraries. [exn-anyhow source](https://docs.rs/exn-anyhow/0.1.0/src/exn_anyhow/lib.rs.html#43-56)

With `(ListenErr, String)` you get lucky and exploit that it's a pattern people generally don't use (so no blanket impl collisions) and that another crate on `anyhow`, `thiserror`, `rootcause`, or a totally different version of Er already knows what to do with a normal error and a `String`. And if the caller IS using Er, the good old `.er()` puts `ListenErr` in their tree and automatically saves your report alongside it:

```rust
radio::listen(85).er(|| StartErr::listen(85))?;
```

Putting the report in a tuple also makes throwing it away impossible to do by mistake. If your caller has a `From<ListenErr>` impl and tries `radio::listen(85)?`, it won't compile because the report is sitting right there next to the error. To drop the report, you have to actually type it out:

```rust
radio::listen(85).map_err(|(error, _)| error)?;
```

Now you can literally see the spot where diagnostics get tossed in the trash (and my dreams).

This also pushes you to put real data on your public errors. If the caller needs `io::ErrorKind` to decide whether to retry, put it on `ListenErr`. Once something only lives in the report string, callers are back to parsing text.

## Why is `.er()` so short?

If adding context is annoying to type, people won't do it. Typing `.change_context_lazy(|| PortErr { input: input.to_owned() })?` on every fallible call is gonna be 85% of your code and next thing you know you're back to lazy `#[from]` enums or bare `?` with zero context.

That's why `.er()` is short, and why `#[derive(Er)]` generates the field helpers and infers the error type from your return type.

```rust
let port = input.parse::<u16>().er(input)?;
read_config_file(path).er((port, enabled))?;
```

You might think the name is stupid, that's fine it is still recognizably 'error', is 2 letters which means it can trivially be prefixed on Er types, which means you don't get collisions and you can do `use er::*;` and get everything you need. This is crucial since developer experience sucks when importing extension traits, and because you don't want to have some er::Result which means you have to preface the std one now (and which makes code dishonest and harder to read vs just seeing ErResult).

## One error type per function (or close to every function)

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

Both calls can fail with `NotFound`. A `From<io::Error>` impl sees the exact same error type and has no idea whether the input file was missing or the output folder didn't exist. Right here at the call site we know both which step failed and which path it was touching.

Giving most fallible functions their own small error type directly above the function stops `?` from silently teleporting the same error up six layers. Every layer has to say what IT was doing, and only what it knows (so you never pass a `PathBuf` down three functions just so an inner error can log it).

One layer up, `.er_with()` turns `CopyErr` into whatever your caller actually cares about. Matching on your own type means adding a new case gives you a compiler error where you handle it, whereas searching for some inner `io::Error` with `er_find` quietly breaks the day someone changes how the file is read.

I like `thiserror` for public enums, I just hate writing outward messages and wiring up `source` fields for every private helper function. Let each function state what it knows and let the tree carry the rest underneath.

## Public vs private boundary (app vs library is old rust meme propaganda)

The more you think about "anyhow for apps thiserror for libs" the more stupid it seems.

Using thiserror because it makes the errors your consumers get at the end, and then doing the same thing for all internal logic, makes no sense.

That's like using unsafe C types all over your code just because you happen to call something at the end that needs it.
