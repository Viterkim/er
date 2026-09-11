# Macros

## What the macros actually do

```rust
#[derive(Er)]
pub struct PortEr {
    pub input: String,
}
```

Is roughly this ordinary Rust:

```rust
use std::{error::Error, fmt};

pub struct PortEr {
    pub input: String,
}
impl PortEr {
    #[must_use]
    pub fn new(input: impl Into<String>) -> Self {
        let input = input.into();
        Self { input }
    }
}
impl fmt::Debug for PortEr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut debug = f.debug_struct("PortEr");
        debug.field("input", &self.input);
        debug.finish()
    }
}
impl fmt::Display for PortEr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}
impl Error for PortEr {}
```

The derive writes those impls. `.er()` comes from the lib. (.er() is implemented on Errors, and you import it as an extension trait).

Enums get a constructor per variant.

Fields use Debug by default, including foreign types. `#[er(format = "{input}")]` if you want options `skip` leaves a field out, `censor` prints japanese styled.

`ErFormat` gives the same constructors and Debug/Display, but no Error. Useful for structs inside your error.

`er_all!(BatchEr::new, [a(), b()])` converts failures separately, so their types can differ. A Vec already has matching types. Both keep failures and drop successes. Roughly:

```rust
let mut errors = Vec::new();
for result in results {
    match result {
        Ok(value) => drop(value),
        Err(error) => errors.push(error.into_er_node()),
    }
}

if errors.is_empty() {
    Ok(())
} else {
    Err(ErTree::new(parent(), errors))
}
```

## Constructors

Both `Er` and `ErFormat` make these. Put `#[er(no_constructors)]` on the struct/enum to write your own instead. It skips `new` and all variant constructors, not `.er_wrap()`.

Unsized fields need `no_constructors` too; these constructors pass fields and return `Self` by value.

Variants get snakecase names. Args follow field order. Strings accept `&str`, numbers keep their exact type. `#[er(exact)]` asks for the field's exact type too.

Heres some examples of writing some stuff and getting some bullshit out. 

```rust,ignore
// You write this
#[derive(Er)]
pub struct StartEr;
// You get this 
impl StartEr {
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

// You write this
#[derive(Er)]
pub struct PortEr(pub String);
// You get this 
impl PortEr {
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        let value = value.into();
        Self(value)
    }
}

// You write this
#[derive(Er)]
pub struct FindEr {
    pub query: String,
    pub account: u64,
}
// You get this 
impl FindEr {
    #[must_use]
    pub fn new(query: impl Into<String>, account: u64) -> Self {
        let query = query.into();
        Self { query, account }
    }
}

// You write this
#[derive(Er)]
pub enum ModeEr {
    Missing,
    Unknown { input: String },
}
// You get this 
impl ModeEr {
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

## Why the tree doesn't implement Debug

It's just a way to force the user (you) to pick between a report/top error, it's very easy to accidentally do a error!("{er:?}") or error!("{er}") and get a random result you did not expect.

```rust
#[derive(Er)]
pub struct PortEr;
pub fn read_port(input: &str) -> Er<u16, PortEr> {
    input.parse().er(PortEr::new)
}

pub fn main() {
    let port = read_port("85").er_report().unwrap();
    println!("{port}");

    let tree = read_port("fakenumber").unwrap_err();
    println!("{}", tree.er_top());
    println!("{}", tree.er_report());
}
```

You probably thought `unwrap()` and `expect()` needed Error right(so did i, but it doesn't matter we don't even impl it for Report)?

But it's actually Debug, so do `.er_report()` or `.er_top()` first.

A `main` can return `Result<(), ErReport<AppEr>>` or `Result<(), ErTop<AppEr>>`.

Just want your own typed error? `tree.top`. Still your type, no downcast.

## Wrap

When you need to implement a trait for your error type it would usally be fine, but in Er (and other crates) you don't own `ErTree`. [Rust's orphan rule](https://doc.rust-lang.org/reference/items/implementations.html#trait-implementation-coherence).

That means its GG because in rust you can't implement a foreign trait for a foreign type.

So Wrap is just a wrapper around ErTree:
```rust
// You write this
#[derive(Er)]
#[er(wrap(name = BaseErWrap))]
pub struct BaseEr;

// You get this (plus a LOT of helpers, so you can treat it like the normal case)
pub struct BaseErWrap {
    pub tree: ErTree<BaseEr>,
}
```

Lets look at the `Axum` example, where they own the trait `IntoResponse`.

```rust
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
#[derive(Er)]
#[er(wrap(name = BaseErWrap))] // Is the default name, but written out here for the example
pub struct BaseEr {
    pub input: String,
}

// You can From<NonWrapped> for you <Wrapped> as seen here with '?'
pub fn read_port(input: &str) -> Result<u16, BaseErWrap> {
    let port = input.parse().er(|| BaseEr::new(input))?;
    Ok(port)
}

// The trait from Axum 
impl IntoResponse for BaseErWrap {
    fn into_response(self) -> Response {
        let response = (StatusCode::BAD_REQUEST, "invalid port");
        response.into_response()
    }
}
```

Wrap still works with `.er()`, `.er_report()` and `.er_top()`. 

Want Display/Debug on the Wrap itself? Add `output = report` or `output = top`. Axum doesn't care.

[The Axum integration](../../integrations/axum/src/lib.rs) shows another example.

## Wrap with Error

!!WARNING!! ONLY add std_error if the foreign trait needs Error on the Wrap itself (Axum doesn't). You HAVE to use `.er_from_wrap()` after that because `.er()` hides sub errors from find!

If the trait needs `Error` on the Wrap itself, add `std_error` with `output`.

```rust
#[derive(Er)]
#[er(wrap(name = HandlerError, output = report, std_error))]
pub struct HandlerEr;
```

When you get its Result back, use this instead of `.er(...)`

```rust
handler(input).er_from_wrap(RequestEr::new)
```

For aggregation, take the tree out first with `.er_tree()` on the Result, or `.tree` on the Wrap.

This needs your actual Wrap back. If another lib boxed it, you need to downcast it first.
