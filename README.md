# Er, errors with less r

Convenient error handling with very easy(i hope) ways to add context at every step.

First checkout the small example below, then for more examples try checking out [Examples](./er/docs/examples.md).

There's also an [extensive comparison on error libraries / opinion piece](er/docs/err-lib-comparisons.md) for a bigger comparison/opinion piece with examples. (Obviously biased, but trying to be fair with examples).

## Example

What does the function caller care about? How can you be 'kinda' forced to add context? What would be nice to see in error logs at 03:00?(no AM for you americans). 

Basic example where we add context and keep the original error (plus where we called `.er()`).
```rust
use er::*;

#[derive(Er)]
pub struct PortEr(pub String);
pub fn read_port(input: &str) -> Er<u16, PortEr> {
    let port: u16 = input.parse().er(|| PortEr::new(input))?;
    Ok(port)
}
```
Printing with `.er_report()` 
```text
PortEr("fakenumber") @ er/runnable_examples/context.rs:6:35
`- invalid digit found in string @ er/runnable_examples/context.rs:6:35
```
Or printing with `.er_top()`
```text
PortEr("fakenumber")
```

Here we 'yeet' it up with an empty unit struct, while still adding the file name/line number. (Which is why we don't reuse the error type, to force new context).
```rust
#[derive(Er)]
pub struct ReadEr;
pub fn read() -> Er<(), ReadEr> {
    let _port = read_port("fakenumber").er(ReadEr::new)?;
    Ok(())
}
```
```text
ReadEr @ er/runnable_examples/context.rs:13:41
`- PortEr("fakenumber") @ er/runnable_examples/context.rs:6:35
   `- invalid digit found in string @ er/runnable_examples/context.rs:6:35
```

And when things get spicy:
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
        read_config("HaandboldFuglen", "HaandboldFuglen_token", "nope", Some("microsoftjavaakacsharp")),
        read_config("ComputerKatten", "ComputerKatten_token", "85", None),
    ])?;
    Ok(())
}
```
```text
StartupEr @ er/runnable_examples/context.rs:55:5
|- ConfigEr { machine: "HaandboldFuglen", token: *CENSORED* } @ er/runnable_examples/context.rs:46:21
|  `- PortEr("nope") @ er/runnable_examples/context.rs:6:35
|     `- invalid digit found in string @ er/runnable_examples/context.rs:6:35
`- ConfigEr { machine: "ComputerKatten", token: *CENSORED* } @ er/runnable_examples/context.rs:47:21
   `- ModeEr::Missing @ er/runnable_examples/context.rs:22:21
```

## How to use + convenience

The TLDR is `#[derive(Er)]YourEr + return Er<(), YourEr> + .er(||)?;`

AVOID `map_err(|error|)` for adding context! You're gonna nuke the tree(unless you manually handle it). Use `.er(...)`.

The error tree(which is not the Report) is `ErTree<YourEr>` which is used as `Result<T, ErTree<YourEr>>` and you're gonna write that a lot so the user facing short form is `Er<T, YourEr>`.

Each function that handles errors should have their own little Er type, this is to force readding context. (If you just did 'EverythingEr' you could just '?' everywhere).
```rust
#[derive(Er)]
pub struct StartupEr;
pub fn startup() -> Er<(), StartupEr> {
    thing_that_goes_badly().er(StartupEr::new)
}
```

On results/options/trees, `.er(...)` takes a closure, and only construct on errors, so `OtherEr::new` works, but if you have arguments you'll need `|| ThirdEr::new(arg1, arg2)`. You don't have to worry about `.to_string()` when it's relevant it takes `Into<T>`, primitives are also handled without intoing them blindly.

```rust
#[derive(Er)]
pub enum BingoEr {
    // BingoEr::parse() generated
    Parse { input: String },
}
pub fn bingo(input: &str) -> Er<u8, BingoEr> {
    some_func().er(|| BingoEr::parse(input))
}
```

## Why?

You don't need a source field manually, writing out wrappers, or an enum of every suberror imaginable. Avoid the whole big error spaghetti pyramid.

It's purposely made to force you to pick between report/top error. (Display/debug for error report/top error is confusing and tribal knowledge).

Slightly exaggerated, i want to avoid this: `thing.add_lazy_context_and_its_tuesday(|something_here| #[now_theres_a_macro_here_for_some_reason] YouGetThePoint { a: "85".to_string() } )`, i just wanna do `.er(||)`.

Give each func its own small error type, a unit error is enough most of the time (`src_location` is there by default).

Having an easy to use macro with the defaults you want, is the thing that makes each function have their own little `Er` type not be painful.

## The macro (it does what it do)

If you have other structs and you want display/debug in the same way you can use `#[derive(ErFormat)]`.

You can use `#[er(skip)]` to leave out a field or `#[er(censor)]` which shows up as `*CENSORED*` (Stored value is NOT removed!).

If you like the defaults, and want to type out your own error messages you can do `#[er(format = "couldn't read {path:?}")]` on the struct or enum variant (does Debug and Display).

## External traits / crates
If a framework needs you to implement its trait on a local type (hello axum IntoResponse) `#[er(wrap)]` generates that wrapper with a public `tree` field.

Look into `integrations/axum/src/lib.rs` for a fresh wrapped error, in short you can do this `return Err(MyEr::new().er_wrap());`.

`.er(...)?` converts into Wrap when that's your return type. Coming back? Use [`.er_tree().er(...)`](er/docs/examples.md#wrap-and-back).

Anyhow works without a feature or adapter. Its normal boxed conversion works, BUT `integrations/anyhow/src/lib.rs` shows the conversion choice you gotta do.

## Extra info / Features

`no_std + alloc`

3 features, 2 normal `macros, src_locations` and 1 for dev/testing `test`.

Normal:

`macros`, enabled by default, gives you `#[derive(Er)]` and `#[derive(ErFormat)]`.

`src_locations`, enabled by default (what line and where).

Dev:

`test`, gives you `TestEr` and `.t_er()?`, off by default (for tests, set 'er' as a dev dependency).
```toml
[dev-dependencies]
er = { version = "0.1", features = ["test"] }
```

## Disclaimer

The macro portions were heavily gippity assisted.
