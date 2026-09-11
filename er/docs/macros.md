# Macros 

## Why Wrap exists

Axum owns `IntoResponse`, Er owns `ErTree`. Your app can't implement one on the other, even with your error inside. [Rust's orphan rule](https://doc.rust-lang.org/reference/items/implementations.html#trait-implementation-coherence).

So give it a type your app owns:

```rust
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
#[derive(Er)]
#[er(wrap(name = ApiError))]
pub struct PortEr {
    pub input: String,
}
pub fn read_port(input: &str) -> Result<String, ApiError> {
    let port: u16 = input.parse().er(|| PortEr::new(input))?;
    Ok(port.to_string())
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (StatusCode::BAD_REQUEST, "invalid port").into_response()
    }
}
```

`ApiError` owns the tree, `?` puts it there. Other functions can keep returning `Er`.

Keep the report for your logs, send the client what you meant to expose. Top text can contain private data too. [The integration example](../../integrations/axum/src/lib.rs) sends top text and picks a status from the cause.

Axum doesn't need Display, Debug or Error. If another lib does, add `output = report` or `output = top`.

Back to Er? Use [`.er_tree().er(...)`](examples.md#wrap-and-back). A Wrap with `output` can go straight through `.er(...)`, but then searches stop at the wrapper.

## Why the tree doesn't implement Debug

It's just a way to force the user (you) to pick between a report/only the top error, it's very easy to accidentally do a error!("{er:?}") or error!("{er}") and get a random result you did not expect.

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

`unwrap()` and `expect()` need Debug for the panic message. Pick `.er_report()` or `.er_top()` first.

A `main` can return `Result<(), ErReport<AppEr>>` or `Result<(), ErTop<AppEr>>`.

An Error impl on the tree would also clash with its structural conversion. Owned top/report implement Error when the root is `Error + 'static`.

Just want your own typed error? `tree.top`. Still your type, no downcast.

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

The derive writes those impls. `.er()` comes from the lib.

Enums get a constructor per variant. Fields use Debug by default, including foreign types. `#[er(format = "{input}")]` if you want to decide yourself. `skip` leaves a field out, `censor` prints japanese styled.

`ErFormat` generates just the matching Debug and Display impls, useful for structs inside your error.

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

## #[derive(Er)] constructors/helpers

Variants get snakecase names. Args follow field order. Strings accept `&str`, numbers keep their exact type. `#[er(exact)]` asks for the field's exact type too.

Type and then what gets generated ish:

```rust,ignore
#[derive(Er)]
pub struct StartEr;
// Gives this
impl StartEr {
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

#[derive(Er)]
pub struct PortEr(pub String);
// Gives this
impl PortEr {
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        let value = value.into();
        Self(value)
    }
}

#[derive(Er)]
pub struct FindEr {
    pub query: String,
    pub account: u64,
}
// Gives this
impl FindEr {
    #[must_use]
    pub fn new(query: impl Into<String>, account: u64) -> Self {
        let query = query.into();
        Self { query, account }
    }
}

#[derive(Er)]
pub enum ModeEr {
    Missing,
    Unknown { input: String },
}
// Gives this
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

Calls look like this:

```rust,ignore
let _ = StartEr::new();
let _ = PortEr::new("fakenumber");
let _ = FindEr::new("HaandboldFuglen", 85);
let _ = ModeEr::missing();
let _ = ModeEr::unknown("ComputerKatten");
```
