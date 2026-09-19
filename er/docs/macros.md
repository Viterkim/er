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

The derive writes those impls. `.er()` comes from the lib. (.er() is implemented on Errors, and you import it as an extension trait).

Enums get a constructor per variant.

Fields use Debug by default, including foreign types. `#[er(format = "{input}")]` if you want options `skip` leaves a field out, `censor` prints japanese styled.

`ErFormat` gives the same constructors and Debug/Display, but no Error. Useful for structs inside your error.

`er_all!((), [a(), b()])` runs both, keeps the errors and drops the oks. They can be different types in that list (a Vec needs them to match). The parent only gets made if something failed.

## Constructors

Both `Er` and `ErFormat` make these. Put `#[er(no_constructors)]` on the struct/enum to write your own instead. It skips `new` and all variant constructors, not `.er_wrap()`.

For an empty struct, use `.er(())`. `no_constructors` turns that off too.

For a struct with fields, `.er(|| path)` lets you skip its name at the call site too:

```rust
#[derive(Er)]
pub struct FileErr(pub PathBuf);

fs::read_to_string(path).er(|| path)?;
```

For more fields, use a tuple in field order: `.er(|| (machine, token))`. Enums still need the variant name, like `.er(|| ModeErr::missing(input))`.

If Rust can't tell which error you mean, name it: `.er::<FileErr>(|| path)`. You'll usually need that on the first `.er` when you chain two of them.

This works with `er-macros` on its own too. It makes the field conversions when you derive `Er`, and `no_constructors` skips those as well.

Variants get snakecase names. Args follow field order. Strings accept `&str`, numbers keep their exact type. `#[er(exact)]` asks for the field's exact type too.

Heres some examples of writing some stuff and getting some bullshit out. 

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

When you need to implement a trait for your error type it would usally be fine, but in Er (and other crates) you don't own `ErTree`. [Rust's orphan rule](https://doc.rust-lang.org/reference/items/implementations.html#trait-implementation-coherence).

That means its GG because in rust you can't implement a foreign trait for a foreign type.

So Wrap is just a wrapper around ErTree:
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

Lets look at the `Axum` example, where they own the trait `IntoResponse`.

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

// You can From<NonWrapped> for you <Wrapped> as seen here with '?'
pub fn read_port(input: &str) -> Result<u16, BaseErrWrap> {
    let port = input.parse().er(|| input)?;
    Ok(port)
}

// The trait from Axum 
impl IntoResponse for BaseErrWrap {
    fn into_response(self) -> Response {
        let response = (StatusCode::BAD_REQUEST, "invalid port");
        response.into_response()
    }
}
```

Wrap still works with `.er()`, `.er_report()` and `.er_top()`. 

Want Display/Debug on the Wrap itself? Add `output = report` or `output = top`. Axum doesn't care.

[The Axum integration](../../integrations/axum/src/lib.rs) shows another example.

## Wrap with std_err

!WARNING! ONLY add std_error if the foreign trait needs Error on the Wrap itself (Axum doesn't). You HAVE to use `.er_wrap()` after that because `.er()` hides sub errors from find!

If the trait needs `Error` on the Wrap itself, add `std_error` with `output`.

```rust
#[derive(Er)]
#[er(wrap(name = HandlerError, output = report, std_error))]
pub struct HandlerErr;

pub fn handler(input: &str) -> Result<u16, HandlerError> {
    let port = input.parse().er(())?;
    Ok(port)
}
```

When you get its Result back, use this instead of `.er()`

```rust
#[derive(Er)]
pub struct RequestErr;
pub fn request(input: &str) -> Er<u16, RequestErr> {
    handler(input).er_wrap(RequestErr::new)
}
```

For aggregation, take the tree out first with `.er_tree()` on the Result, or `.tree` on the Wrap.

This needs your actual Wrap back. If another lib boxed it, you need to downcast it first.
