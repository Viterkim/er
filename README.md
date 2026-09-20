# Er, errors with less r

Convenient error handling with very easy(i hope) ways to add typed context at every step.

Made for when stuff goes wrong in production at 03:00(no AM for you americans), for what is nice to see in your errors/logs, for saving you hours of debugging, and what is not annoying to type...

[Examples and patterns to use](./er/docs/examples.md).

[Simple error comparison (basic usage, context, output)](er/docs/simple-error-comparison.md).

[Tricky error comparison (foreign errors, own errors, the original error, string context, typed context, using / consuming, public boundary)](er/docs/tricky-error-comparison.md).

Add it with:
```toml
[dependencies]
er = "0.2"
```

## Initial Example

Use `.er` on basically any result/error/option/tree, even different types.

You add relevant context(or none), Er keeps the original error and adds line number and file name.

```rust
use er::*;
use std::{fs::read_to_string, path::PathBuf};

#[derive(Er)]
pub struct FileErr(pub PathBuf);
pub fn read_file(path: &str) -> Er<String, FileErr> {
    let text = read_to_string(path).er(|_| path)?;
    Ok(text)
}
```
Er turns the `&str` into a `PathBuf` only on failure (And we keep the `io::Error`).

`.er_report()` gives us:
```text
FileErr("/tmp/file.txt") @ er/runnable_examples/context.rs:7:37
`- No such file or directory (os error 2)
```

We can also just 'yeet' it up with an empty struct, still adding the implicit context (where it happened).
```rust
#[derive(Er)]
pub struct AnalyzeErr;
pub fn analyze() -> Er<String, AnalyzeErr> {
    // Serious analysis happening right now
    read_file("/tmp/file.txt").er(())
}
```
```text
AnalyzeErr @ er/runnable_examples/context.rs:14:32
`- FileErr("/tmp/file.txt") @ er/runnable_examples/context.rs:7:37
   `- No such file or directory (os error 2)
```

And when things get spicy
```rust
#[derive(Er)]
pub struct ConfigErr {
    pub machine: String,
    #[er(censor)]
    pub token: String,
}
pub fn read_config(machine: &str, token: &str, port: &str, mode: Option<&str>) -> Er<(), ConfigErr> {
    // add local context
    let e = |_| (machine, token);

    // 3 different types
    authenticate(machine, token).er(e)?;
    read_port(port).er(e)?;
    read_mode(mode).er(e)?;

    Ok(())
}
```

And aggregation/collection
```rust
#[derive(Er)]
pub struct StartupErr(pub String);
pub fn startup() -> Er<(), StartupErr> {
    er_all!(|_| "some config checks failed", [
        read_config("HaandboldFuglen", "HaandboldFuglen_token", "aint_even_a_number_cmon_man", Some("microsoftjavaakacsharp")),
        read_config("ComputerKatten", "ComputerKatten_token", "85", None),
    ])?;
    Ok(())
}
```

Report printed `.er_report()`:
```text
StartupErr("some config checks failed") @ er/runnable_examples/context.rs:69:5
|- ConfigErr { machine: "HaandboldFuglen", token: *CENSORED* } @ er/runnable_examples/context.rs:60:21
|  `- PortErr { invalid_port: "aint_even_a_number_cmon_man" } @ er/runnable_examples/context.rs:22:35
|     `- invalid digit found in string
`- ConfigErr { machine: "ComputerKatten", token: *CENSORED* } @ er/runnable_examples/context.rs:61:21
   `- ModeErr::MissingMode @ er/runnable_examples/context.rs:31:21
```

Top error printed with `.er_top()`:
```text
StartupErr("some config checks failed")
```

Printing those:
```rust
if let Err(error) = startup() {
    println!("{}", error.er_report());
    println!("{}", error.er_top());
}
```

## How to use

The TLDR: make a `NameErr` with `#[derive(Er)]`, return `Er<T, NameErr>`, and add context with `.er()`.

Each function that handles errors should have THEIR OWN little `NameErr` type, this is to force readding context. If you just used an 'EverythingErr' you could '?' everywhere(No new line info added).

In many cases unit structs are enough, ONLY add context where it makes sense (usually small local things that's dynamic).

For empty structs use `.er(())`.

For structs use `.er(|_| arg1)` or `.er(|_| (arg1, arg2))`.

For enums use `.er(EnumErr::variant_name)` and `.er(|| EnumErr::variant_name2(arg1))`.

If you NEED a value from the old error, use `.er_with(|t| NewErr::new(t.top.code))`. It keeps that old error in the tree too. A quick `map_err` can accidentally nuke it.

`.er_with()` is the annoying explicit case, the convenient `|_|` pattern i can't get to work for that. 

## Why?

Convenience for actually typing out stuff yourself, and to avoid having good error handling take up 70% of the line, which often means you avoid doing it.

I believe that people do worse error handling because the ergonomics are bad. I'll even argue it reads better as well once you know it. People take what they know for granted, manual `.map_err(||)` everywhere is nuts.

Exaggerated but i want to avoid: `thing.add_lazy_context_and_its_tuesday(|something_here| #[now_theres_a_macro_here_for_some_reason] YouGetThePoint { a: "85".to_string() } )`.

Forcing you to pick between report/top error. (Display/Debug meaning report is confusing and tribal knowledge).

Having an easy to use macro with the defaults you want, is the thing that makes each function have their own little `Er` type not be painful. And it means we don't have to rely on thiserror, and we can add convenience via the macro.

## Docs

[Changelog](er/docs/changelog.md).

[Features](er/docs/features.md).

[Macros](er/docs/macros.md).

[Weird cases](er/docs/extra/weird-cases.md).

[Performance](er/docs/extra/performance.md).

## Links

[Github Repo](https://github.com/Viterkim/er)

[Docs.rs](https://docs.rs/er/latest/er/)

[Crates.io for the lib](https://crates.io/crates/er)

[Crates.io for the macros](https://crates.io/crates/er-macros/)
