# Macros

Checkout the main examples.md doc for patterns to use, this is mostly just for the macros specifically. 

## Constructors

Both `Er` and `ErFormat` generate `new` for structs and snake_case methods for enum variants. The arguments follow field order, with conversions such as `&str` into `String` done for you. Primitive numbers take their exact type so integer literals pick up the field's type, and you can put `#[er(exact)]` on any field if you want it to take its exact type without `Into`.

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

`#[er(src_location)]` fills a standard `Location` reference at construction, leaving that field out of the arguments (works without the `src_locations` feature too). You'll want to pass it in a closure like `.er(|| LocErr::new())` so the location points to your call site.

The tuple helpers use those same field conversions when an error happens. On struct fields, `#[er(default)]` lets the tuple stop early, the marked fields go last and the ones you leave out get `Default::default()`, or your expression with `#[er(default = make_data())]`. `new(...)` still takes every field.

`#[er(no_constructors)]` turns off the named constructors and the struct's field helpers, including `.er(())`.

The derives call er's own items through `::er`, so a renamed er needs `#[er(crate = other_name)]`.

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
    let port = input.parse().er(input)?;
    Ok(port)
}

impl IntoResponse for BaseErrWrap {
    fn into_response(self) -> Response {
        let response = (StatusCode::BAD_REQUEST, "invalid port");
        response.into_response()
    }
}
```

Wrap and its Results still work with `.er()` and `.er_report()`. `.into()` works too, from a plain error or a tree.

If you need Display/Debug on the Wrap itself you can add `output = report` or `output = top` (Axum doesn't care).

If the `er` crate has another name, give Wrap its path with `#[er(crate = other_name, wrap)]`.

## Source

`#[er(source)]` exposes one cause per struct or variant. Er prints the cause below the error, so it leaves the field out of the parent's default Display and Debug. `#[er(source, show_in_parent)]` prints it inside the parent too, handy when someone prints just that error. An explicit format can use the field as well.

Works with `Option`, `Box`, `Rc`, `Arc` and `ErShared`. For a pointer type alias like `type Cause<E> = Arc<E>`, add `#[er(source(deref))]`.

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

Fields are debug printed by default, foreign types too. In `#[er(format = "{input}")]`, `input` uses Display, or the classic `{input:?}` for Debug. Templates can access nested fields too, like `{oldest.0}` or `{details.message}`. `skip` leaves a field out, `censor` prints `*CENSORED*`.

`format` sets both Display and Debug, or you can override just one with `display` or `debug` and leave the other alone:

```rust
#[derive(Er)]
#[er(display = "couldn't read {path:?}")]
pub struct ReadErr {
    pub path: PathBuf,
    pub attempts: u8,
}
```

Display prints the short message, Debug still prints `ReadErr { path: ..., attempts: ... }`. An explicit `display` or `debug` also overrides a shared `format`, regardless of attribute order. On enums you can put string formats on each variant that needs one.

You can use `display_alt` for `{err:#}` while normal Display and Debug keep their output:

```rust
#[derive(Er)]
#[er(format = "couldn't connect", display_alt = "couldn't connect to {host}, tried {attempts} times")]
pub struct ConnectErr {
    pub host: String,
    pub attempts: u8,
}
```

`{err}` and both Debug forms print `couldn't connect`. `{err:#}` prints `couldn't connect to db.local, tried 3 times`. Enum variants inherit it unless they set their own.

Each setting can also take a formatting function instead of a string, like `#[er(display_alt = Self::details)]`. It gets `&Self` and `&mut fmt::Formatter<'_>` and returns `fmt::Result`.

`ErFormat` gives the same constructors and Debug/Display, but no Error. Useful for structs inside your error.

## Wrap with std_error

! WARNING ! Only add `std_error` with `output` if the foreign trait needs `Error` on the Wrap itself (Axum doesn't). When that Wrap comes back, use the Wrap helpers below or `.er_find()` won't see the errors inside it!

```rust
#[derive(Er)]
#[er(wrap(name = HandlerError, output = report, std_error))]
pub struct HandlerErr;

pub fn handler(input: &str) -> Result<u16, HandlerError> {
    let port = input.parse().er(())?;
    Ok(port)
}
```

When you get its Result back, you can use this instead of `.er()`:

```rust
#[derive(Er)]
pub struct RequestErr {
    pub input: String,
}

pub fn request(input: &str) -> ErResult<u16, RequestErr> {
    handler(input).er_wrap(input)
}
```

If you need the old top, you can use `.er_with_wrap(|err| fields)`.

This needs your actual Wrap back. If another lib boxed it, downcast it first.
