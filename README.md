# Er, errors with less r

Convenient error handling with very easy(i hope) ways to add typed context at every step.

The main focus is caring about what the function caller cares about, how can you 'kinda' be forced to add context, and what would be nice to see in error logs at 03:00?(no AM for you americans). 

For more examples look at [Most common examples / use cases](./er/docs/examples.md).

There's also an [extensive comparison on error libraries / opinion piece](er/docs/err-lib-comparisons.md) for a bigger comparison/opinion piece with examples. (Obviously biased, but trying to be fair with examples).

Add it with:
```toml
[dependencies]
er = "0.1"
```

## Initial Example

Use `.er` on basically any result/error/option/tree, even different types. 

You add relevant context(or none), Er keeps the original error and adds line number and file name.

Basic example where we add context.
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

And when things get spicy ([the other helpers](er/runnable_examples/context.rs)):
```rust
#[derive(Er)]
pub struct ConfigEr {
    pub machine: String,
    #[er(censor)]
    pub token: String,
}
pub fn read_config(machine: &str, token: &str, port: &str, mode: Option<&str>) -> Er<(), ConfigEr> {
    // add local context (new() generated and ordered)
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
    // Aggregate different results (even different types)
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

The TLDR is `#[derive(Er)]YourEr + return Er<(), YourEr> + .er(||)?;`

AVOID `map_err(|error|)` for adding context! You're gonna nuke the tree(unless you manually handle it). Use `.er(||)`.

`Er<T, YourEr>` is `Result<T, ErTree<YourEr>>`.

Each function that handles errors should have THEIR OWN little Er type, this is to force readding context. If you just used an 'EverythingEr' you could '?' everywhere without context.

For structs use `.er(|| OtherEr::new(arg1))`, for empty structs use `.er(OtherEr::new)`.

For enums use `.er(|| EnumErr::variant_name(arg1))`, for empty variants use `.er(EnumErr::variant_name)`.

Enums get a constructor per variant.
```rust
#[derive(Er)]
pub enum BingoEr {
    // BingoEr::parse() generated
    Parse { input: String, favorite_number: u32 },
}
pub fn bingo(input: &str) -> Er<u8, BingoEr> {
    input.parse().er(|| BingoEr::parse(input, 85))
}
```

## Why?

You don't add a source field manually, you don't have to write out wrappers, you avoid the god enum of every suberror imaginable(thiserror spaghetti pyramid).

It's designed to force you to pick between report/top error. (Display/Debug meaning report or something else is confusing and tribal knowledge).

Slightly exaggerated, i want to avoid: `thing.add_lazy_context_and_its_tuesday(|something_here| #[now_theres_a_macro_here_for_some_reason] YouGetThePoint { a: "85".to_string() } )`, i just wanna do `.er(||)`.

Having an easy to use macro with the defaults you want, is the thing that makes each function have their own little `Er` type not be painful.

## The macro (it does what it do)

If you have other structs and you want display/debug in the same way: `#[derive(ErFormat)]`.

You can use `#[er(skip)]` to leave out a field or `#[er(censor)]` which shows up as `*CENSORED*` (Stored value is NOT removed!).

If you HATE the defaults and want to type out your own messages: `#[er(format = "couldn't read {path:?}")]`. 

## Features / Extra

The crate is `no_std` but requires `alloc`

default features: `macros, src_locations`

non-default features: `small_path_src`

non-default dev/testing feature: `test`

### macros

Convenience macros that kinda is the point of Er, you get `#[derive(Er)]` and `#[derive(ErFormat)]` etc.

### src_locations

Enables capturing the file path, line number and column number at compile time.

### small_path_src, off by default

Usually you'll just get `src/lib.rs` or `engine/src/lib.rs` in a workspace(so you WONT need this), but on some setups if you use paths for dependencies, like a local crate on your pc, then you'll get the full path `/the/magic/src/folder/that/was/supposed/to/be/secret/src/a.rs`.

So this turns it into `secret/src/a.rs`. It just looks for the last `src` and gives you the path one step back from that.

If you don't want to restructure stuff you can enable it BUT! it only affects it when PRINTING! It is STILL in your binary.

### test, for dev dependencies, off by default

`TestEr` and `.t_er()?` for tests. 

### Examples

```toml
[dependencies]
er = { version = "0.1", features = ["small_path_src"] }

[dev-dependencies]
er = { version = "0.1", features = ["test"] }
```

```toml
[dependencies]
er = { version = "0.1", default-features = false, features = ["src_locations"] }
```

## Disclaimer

The macro portions were heavily gippity assisted.
