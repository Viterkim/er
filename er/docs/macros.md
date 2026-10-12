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

Fields are debug printed by default, foreign types too. In `#[er(format = "{input}")]`, `input` uses Display, or the classic `{input:?}` for Debug. Templates can access nested fields too, like `{oldest.0}` or `{details.message}`. `skip` leaves a field out, `censor` prints `*CENSORED*`.

`format` sets both Display and Debug. Override either with `display` or `debug`, the other keeps its usual output:

```rust
#[derive(Er)]
#[er(display = "couldn't read {path:?}")]
pub struct ReadErr {
    pub path: PathBuf,
    pub attempts: u8,
}
```

Display prints the short message, Debug still prints `ReadErr { path: ..., attempts: ... }`. An explicit `display` or `debug` also overrides a shared `format`, regardless of attribute order. The overrides work on structs, enums and individual variants.

Use `display_alt` for `{err:#}`, while ordinary Display and Debug keep their output:

```rust
#[derive(Er)]
#[er(format = "operation failed", display_alt = "{diagnostics}")]
pub struct SavedErr {
    pub diagnostics: String,
}
```

`{err}` and both Debug forms print `operation failed`. `{err:#}` prints the saved diagnostics. Without `display_alt`, alternate formatting keeps working as before. Enum variants inherit it unless they set their own.

Each setting can take a formatting function instead of a string, like `#[er(display_alt = Self::details)]`. It receives `&Self` and `&mut fmt::Formatter<'_>` and returns `fmt::Result`. Conditions such as omitting empty diagnostics stay ordinary Rust.

`ErFormat` gives the same constructors and Debug/Display, but no Error. Useful for structs inside your error.

## Constructors

Both `Er` and `ErFormat` generate `new` for structs and snake_case methods for enum variants. The arguments follow field order, with conversions such as `&str` into `String` done for you. Numbers already take their exact type, and `#[er(exact)]` does that for another field too. Here's what those methods look like:

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

`#[er(src_location)]` fills a standard `Location` reference at construction, leaving that field out of the arguments. It works without the runtime `src_locations` feature too.

The tuple helpers use those same field conversions when an error happens. `#[er(default)]` adds a form that omits all marked fields using `Default::default()`, or your expression with `#[er(default = make_data())]`. The full tuple and `new(...)` still take every ordinary field.

`#[er(no_constructors)]` turns off the named constructors and the struct's field helpers, including `.er(())`. A requested Wrap constructor stays available.

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

Wrap and its Results still work with `.er()`, `.er_report()` and `.er_top()`.

Want Display/Debug on the Wrap itself? Add `output = report` or `output = top`. Axum doesn't care.

If the `er` crate has another name, give Wrap its path with `#[er(crate = other_name, wrap)]`.

## Public errors

At the public boundary, keep the diagnostics you need in an ordinary error (they get a normal error and don't depend on 'er').

```rust
#[derive(Er)]
#[er(format = "couldn't open {path:?}", display_alt = "{diagnostics}")]
pub struct OpenError {
    pub path: PathBuf,
    #[er(into_top, source)]
    pub cause: std::io::Error,
    #[er(into_report_string)]
    pub diagnostics: String,
}
```

`into_top` moves the typed top into `cause`, and `into_report_string` saves the report. `source` exposes the cause through Rust's `Error::source()`. And we offer full diagnostics with `{err:#}` (or accesing the data on the type).

Return the report from `.er_into()` and it fills the marked fields. Any remaining fields go beside it, in field order:

```rust
result.er_into(|report| (report, path)) // OpenError above
result.er_into(|report| report) // No remaining fields
result.er_into(|report| (report, (request_id, path))) // Several fields
```

The saved string keeps the report's layout. Use `into_report_string(single_line)` to force one line (note that you can do into_snapshot, but be careful as you are then providing a type from 'er' it self which means your consumer know has an indirect depedency, avoid it for consumers who are external to your crate).

You can use `source` without saving a report too, with one source per struct or variant. Box, Rc and Arc expose their contained error, and Option exposes nothing on None. For a pointer alias like `type Cause<E> = Arc<E>`, use `source(deref)`. `skip` only leaves the field out of formatting.

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

When you get its Result back, use this instead of `.er()`:

```rust
#[derive(Er)]
pub struct RequestErr {
    pub input: String,
}

pub fn request(input: &str) -> ErResult<u16, RequestErr> {
    handler(input).er_wrap(|_| input)
}
```

If you need the old top, use `.er_with_wrap(|err| fields)`.

For aggregation, take the tree out first with `.er_tree()` on the Result, or `.tree` on the Wrap.

This needs your actual Wrap back. If another lib boxed it, you need to downcast it first.
