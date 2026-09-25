# Macros

## Generated code

```rust
#[derive(Er)]
pub struct PortErr {
    pub input: String,
}
```

Is roughly this ordinary Rust:

```rust
use std::{error::Error, fmt};

pub struct PortErr {
    pub input: String,
}
impl PortErr {
    #[must_use]
    pub fn new(input: impl Into<String>) -> Self {
        let input = input.into();
        Self { input }
    }
}
impl fmt::Debug for PortErr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut debug = f.debug_struct("PortErr");
        debug.field("input", &self.input);
        debug.finish()
    }
}
impl fmt::Display for PortErr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}
impl Error for PortErr {}
```

The derive writes those impls. `.er()` comes from the `er` lib as an extension trait on errors.

Enums get a constructor per variant.

Fields are debug printed by default, foreign types too. In `#[er(format = "{input}")]`, `input` uses Display, or the classic `{input:?}` for Debug. `skip` leaves a field out, `censor` prints `*CENSORED*`.

`ErFormat` gives the same constructors and Debug/Display, but no Error. Useful for structs inside your error.

`er_all!((), [a(), b()])` runs both, keeps the errors and drops the oks. They can be different types in that list (a Vec needs them to match). The top error only gets made if something failed.

## Constructors

Both `Er` and `ErFormat` make constructors. Put `#[er(no_constructors)]` on the struct/enum to write your own instead. It skips `new` and all variant constructors. For `Er` structs it also skips the field conversions for `|_|`, but `#[er(wrap)]` still makes `MyErr::er_wrap()`.

For an empty struct, use `.er(())`. `no_constructors` turns that off too.

For a struct with fields, `.er(|_| path)` lets you skip typing out the name. The fields are converted only if something fails.

```rust
use er::*;
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Er)]
pub struct FileErr(pub PathBuf);

pub fn read_file(path: &Path) -> Er<String, FileErr> {
    fs::read_to_string(path).er(|_| path)
}
```

For more fields, use a tuple `.er(|_| (machine, token))`.

Enums still need the variant name, like `.er(|| ModeErr::unknown(input))`.

If Rust can't tell which error you mean (usually when chaining), use `.er::<FileErr>(|_| path)`.

Variants get snake_case constructor names. Args follow field order. Strings accept `&str`, numbers keep their exact type. `#[er(exact)]` asks for the field's exact type too.

Examples below of what it spits out.

```rust,ignore
// You write this
#[derive(Er)]
pub struct StartErr;
// You get this
impl StartErr {
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

// You write this
#[derive(Er)]
pub struct PortErr(pub String);
// You get this
impl PortErr {
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        let value = value.into();
        Self(value)
    }
}

// You write this
#[derive(Er)]
pub struct FindErr {
    pub query: String,
    pub account: u64,
}
// You get this
impl FindErr {
    #[must_use]
    pub fn new(query: impl Into<String>, account: u64) -> Self {
        let query = query.into();
        Self { query, account }
    }
}

// You write this
#[derive(Er)]
pub enum ModeErr {
    Missing,
    Unknown { input: String },
}
// You get this
impl ModeErr {
    #[must_use]
    pub fn missing() -> Self {
        Self::Missing
    }

    #[must_use]
    pub fn unknown(input: impl Into<String>) -> Self {
        let input = input.into();
        Self::Unknown { input }
    }
}
```

## Wrap

If you need to implement another crate's trait on the whole error tree, Rust says no: you don't own `ErTree` or that trait. [Orphan rule](https://doc.rust-lang.org/reference/items/implementations.html#trait-implementation-coherence)

Wrap gives you a type you *do* own, around the tree:

```rust
// You write this
#[derive(Er)]
#[er(wrap(name = BaseErrWrap))]
pub struct BaseErr;

// You get this (plus a LOT of helpers, so you can treat it like the normal case)
pub struct BaseErrWrap {
    pub tree: ErTree<BaseErr>,
}
```

Let's look at the `Axum` example, where we own `BaseErrWrap` and can implement Axum's `IntoResponse` for it.

```rust
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};

#[derive(Er)]
#[er(wrap(name = BaseErrWrap))] // Is the default name, but written out here for the example
pub struct BaseErr {
    pub input: String,
}

// ? turns the normal Er tree into our Wrap
pub fn read_port(input: &str) -> Result<u16, BaseErrWrap> {
    let port = input.parse().er(|_| input)?;
    Ok(port)
}

impl IntoResponse for BaseErrWrap {
    fn into_response(self) -> Response {
        let response = (StatusCode::BAD_REQUEST, "invalid port");
        response.into_response()
    }
}
```

Results using Wrap still work with `.er()`, `.er_report()` and `.er_top()`.

Want Display/Debug on the Wrap itself? Add `output = report` or `output = top`. Axum doesn't care.

If the `er` crate has another name, give Wrap its path with `#[er(crate = other_name, wrap)]`.

## Wrap with std_error

! WARNING ! Only add `std_error` with `output` if the foreign trait needs `Error` on the Wrap itself (Axum doesn't). When that Wrap comes back, use `.er_wrap()` instead of `.er()` or `.er_find()` won't see the errors inside it!

```rust
#[derive(Er)]
#[er(wrap(name = HandlerError, output = report, std_error))]
pub struct HandlerErr;

pub fn handler(input: &str) -> Result<u16, HandlerError> {
    let port = input.parse().er(())?;
    Ok(port)
}
```

When you get its Result back, use this instead of `.er()`:

```rust
#[derive(Er)]
pub struct RequestErr {
    pub input: String,
}

pub fn request(input: &str) -> Er<u16, RequestErr> {
    handler(input).er_wrap(|_| input)
}
```

For aggregation, take the tree out first with `.er_tree()` on the Result, or `.tree` on the Wrap.

This needs your actual Wrap back. If another lib boxed it, you need to downcast it first.
