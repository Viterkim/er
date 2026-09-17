# Er, errors with less r

Convenient error handling with very easy ways to add typed context at every step.

The main focus is having good types at every step, and caring about what the caller wants to know. To have fully typed context be convenient and easy to type.

And then maybe most importantly, when stuff goes wrong in production at 03:00(no AM for you americans), what is gonna save you hours of debugging?

[Examples and patterns to use](./er/docs/examples.md).

[Simple error comparison (basic usage, context, output)](er/docs/simple-error-comparison.md).

[Tricky error comparison (foreign errors, own errors, the original error, string context, typed context, using / consuming, public boundary)](er/docs/tricky-error-comparison.md).

[Features](er/docs/features.md).

[Macros](er/docs/macros.md).

Add it with:
```toml
[dependencies]
er = "0.1"
```

## Initial Example

Use `.er` on basically any result/error/option/tree, even different types. 

You add relevant context(or none), Er keeps the original error and adds line number and file name.

```rust
use er::*;
use std::{fs::read_to_string, path::PathBuf};

#[derive(Er)]
pub struct FileEr(pub PathBuf);
pub fn read_file(path: &str) -> Er<String, FileEr> {
    let text = read_to_string(path).er(|| FileEr::new(path))?;
    Ok(text)
}
```
`FileEr::new` is generated, turns the `&str` into a `PathBuf`, and only runs on failures (And we keep the `io::Error`).

`.er_report()` gives us:
```text
FileEr("/tmp/file.txt") @ er/runnable_examples/context.rs:7:37
`- No such file or directory (os error 2) @ er/runnable_examples/context.rs:7:37
```

We can also just 'yeet' it up with an empty struct, still adding the implicit context.
```rust
#[derive(Er)]
pub struct AnalyzeEr;
pub fn analyze() -> Er<String, AnalyzeEr> {
    // Serious analysis happening right now
    read_file("/tmp/file.txt").er(AnalyzeEr::new)
}
```
```text
AnalyzeEr @ er/runnable_examples/context.rs:14:32
`- FileEr("/tmp/file.txt") @ er/runnable_examples/context.rs:7:37
   `- No such file or directory (os error 2) @ er/runnable_examples/context.rs:7:37
```

And when things get spicy
```rust
#[derive(Er)]
pub struct ConfigEr {
    pub machine: String,
    #[er(censor)]
    pub token: String,
}
pub fn read_config(machine: &str, token: &str, port: &str, mode: Option<&str>) -> Er<(), ConfigEr> {
    // add local context
    let er = || ConfigEr::new(machine, token);

    // 3 different types 
    authenticate(machine, token).er(er)?;
    read_port(port).er(er)?;
    read_mode(mode).er(er)?;

    Ok(())
}
```

And aggregation/collection
```rust
#[derive(Er)]
pub struct StartupEr;
pub fn startup() -> Er<(), StartupEr> {
    er_all!(StartupEr::new, [
        read_config("HaandboldFuglen", "HaandboldFuglen_token", "aint_even_a_number_cmon_man", Some("microsoftjavaakacsharp")),
        read_config("ComputerKatten", "ComputerKatten_token", "85", None),
    ])?;
    Ok(())
}
```

Report printed `.er_report()`:
```text
StartupEr @ er/runnable_examples/context.rs:64:5
|- ConfigEr { machine: "HaandboldFuglen", token: *CENSORED* } @ er/runnable_examples/context.rs:55:21
|  `- PortEr { invalid_port: "aint_even_a_number_cmon_man" } @ er/runnable_examples/context.rs:22:35
|     `- invalid digit found in string @ er/runnable_examples/context.rs:22:35
`- ConfigEr { machine: "ComputerKatten", token: *CENSORED* } @ er/runnable_examples/context.rs:56:21
   `- ModeEr::MissingMode @ er/runnable_examples/context.rs:31:21
```

Top error printed with `.er_top()`:
```text
StartupEr
```

Printing those:
```rust
if let Err(error) = startup() {
    println!("{}", error.er_report());
    println!("{}", error.er_top());
}
```

## How to use + convenience

// TODO: viktor redo a bit

The TLDR is `#[derive(Er)]YourEr + return Er<(), YourEr> + .er(||)?;`

AVOID `map_err(|error|)` for adding context! You're gonna nuke the tree(unless you manually handle it). Use `.er(||)`.

`Er<T, YourEr>` is `Result<T, ErTree<YourEr>>`. (You don't need to remember this).

Each function that handles errors should have THEIR OWN little Er type, this is to force readding context. If you just used an 'EverythingEr' you could '?' everywhere without context.

In many cases unit structs are more than enough, add the context where it makes sense, and where it helps, usually small local things that change on runs.

For structs use `.er(|| OtherEr::new(arg1))`, for empty structs use `.er(OtherEr::new)`.

For enums use `.er(|| EnumErr::variant_name(arg1))`, for empty variants use `.er(EnumErr::variant_name)`.

If you NEED a value on the error in the error you are creating now, use `.er_with(|e|)`. It adds it to the tree instead of destroying it: `.er_with(|t| OtherEr::new(t.top.code))` (typed error is `t.top`).

## Why?

Ultra convenience for actually typing out stuff yourself, and for not having the error handling in complex cases take up 70% of the code line. I really believe that people do worse error handling because the ergonomics are bad, why should the typed experience be hard/take up so much code? I'll even argue it reads better as well once you know it (And that's the case with anything, people take what they know for granted).

Concretely and exaggerated but i want to avoid: `thing.add_lazy_context_and_its_tuesday(|something_here| #[now_theres_a_macro_here_for_some_reason] YouGetThePoint { a: "85".to_string() } )`, i just wanna do `.er(||)`.

Being 'somewhat forced' or atleast very inclined to add context, or at the very least having the gap between the lazy version, and the fully typed context with variants be tiny, is very important to actually doing the good thing everywhere.

Forces you to pick between report/top error. (Display/Debug meaning report is confusing and tribal knowledge).

Having an easy to use macro with the defaults you want, is the thing that makes each function have their own little `Er` type not be painful. And it means we don't have to rely on thiserror, and we can add convenience via the macro.

## Links

[Github Repo](https://github.com/Viterkim/er)

[Docs.rs](https://docs.rs/er/latest/er/)

[Crates.io for the lib](https://crates.io/crates/er)

[Crates.io for the macros](https://crates.io/crates/er-macros/)
