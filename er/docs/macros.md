# Macros

## Generated code

```rust
#[derive(Er)]
pub struct PortErr {
    pub input: String,
}
```

Here's roughly what the derive writes.

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

Display prints the short message, Debug still prints `ReadErr { path: ..., attempts: ... }`. An explicit `display` or `debug` also overrides a shared `format`, regardless of attribute order. On enums, set a string `format` on each variant that needs one.

Use `display_alt` for `{err:#}`, while ordinary Display and Debug keep their output:

```rust
#[derive(Er)]
#[er(format = "couldn't connect", display_alt = "couldn't connect to {host}, tried {attempts} times")]
pub struct ConnectErr {
    pub host: String,
    pub attempts: u8,
}
```

`{err}` and both Debug forms print `couldn't connect`. `{err:#}` prints `couldn't connect to db.local, tried 3 times`. Enum variants inherit it unless they set their own.

The field can be an `Option` too. `{hint}` prints what's inside, and on `None` you get the output you'd have without that format, so `{err:#}` goes back to `couldn't connect`:

```rust
#[derive(Er)]
#[er(format = "couldn't connect", display_alt = "couldn't connect, {hint}")]
pub struct ConnectErr {
    pub hint: Option<String>,
}
```

Each setting can take a formatting function instead of a string, `#[er(display_alt = Self::details)]`, handy when an empty hint shouldn't print either. It gets `&Self` and `&mut fmt::Formatter<'_>` and returns `fmt::Result`.

`ErFormat` gives the same constructors and Debug/Display, but no Error. Useful for structs inside your error.

## Constructors

Both `Er` and `ErFormat` generate `new` for structs and snake_case methods for enum variants. The arguments follow field order, with conversions such as `&str` into `String` done for you. Primitive numbers take their exact type, so an integer literal can pick up the field's type. Use `#[er(exact)]` to take one of your own types directly.

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

The tuple helpers use those same field conversions when an error happens. On struct fields, `#[er(default)]` adds a form that omits all marked fields using `Default::default()`, or your expression with `#[er(default = make_data())]`. The full tuple and `new(...)` still take every ordinary field.

`#[er(no_constructors)]` turns off the named constructors and the struct's field helpers, including `.er(())`.

The derives call er's own items through `::er`, so a renamed er needs `#[er(crate = other_name)]`.

## Wrap

Rust won't let you implement another crate's trait on `ErTree`. A Wrap gives you a type you own, with the tree still inside it.

```rust
#[derive(Er)]
#[er(wrap)]
pub struct PortErr {
    pub input: String,
}

pub fn read_port(input: &str) -> Result<u16, PortErrWrap> {
    let port = input.parse().er(input)?;
    Ok(port)
}

impl IntoResponse for PortErrWrap {
    fn into_response(self) -> Response {
        tracing::error!("{}", self.er_report());
        (StatusCode::BAD_REQUEST, "invalid port").into_response()
    }
}
```

The `?` puts the tree into `PortErrWrap`. You can also use `PortErrWrap::from(error)` with a plain error or a tree. Add context when another part of your app calls `read_port` with `.er()` or `.er_with()`. For a public error meant to outlive your Er version, hand out the error and report as a pair. [Public errors](https://github.com/Viterkim/er/blob/main/er/docs/examples.md#public-error)

Give it a different name with `wrap(name = Name)`. A trait that needs Display or Debug on the Wrap can use `output = report` or `output = top`.

## Source

`#[er(source)]` exposes one cause per struct or variant. Er prints the cause below the error, so it leaves the field out of the parent's default Display and Debug. `#[er(source, show_in_parent)]` prints it inside the parent too, handy when someone prints just that error. An explicit format can use the field as well.

Box, Rc and Arc expose the error inside. An Option can leave the source empty. With a generic pointer alias like `type Cause<E> = Arc<E>`, put `#[er(source(deref))]` on the field and write an outer Option as `Option<Cause<E>>`.

The derive only sees the type the way you wrote it. Put that alias inside another pointer (`Box<Cause<E>>`, `&Cause<E>`), or alias a pointer to your own error trait (`type Cause = Arc<dyn DriverError>`), and the pointer itself becomes the source, so `er_find` can't see the error inside it. Write the pointer out in the field for those.

## Wrap with std_error

Only use `std_error` when the foreign trait requires `Error` on the Wrap. It needs `output = top` or `output = report`. Coming back from that API, use `.er_wrap()` to keep the whole tree.

```rust
#[derive(Er)]
#[er(wrap(name = HandlerError, output = report, std_error))]
pub struct HandlerErr;

pub fn handler(input: &str) -> Result<u16, HandlerError> {
    let port = input.parse().er(())?;
    Ok(port)
}
```

Use `.er_wrap()` when you get the Result back. A plain `.er()` boxes the Wrap, so a search won't find the errors inside.

```rust
#[derive(Er)]
pub struct RequestErr {
    pub input: String,
}

pub fn request(input: &str) -> ErResult<u16, RequestErr> {
    handler(input).er_wrap(input)
}
```

`.er_with_wrap(|err| fields)` lets you read the old top. Both helpers also work on a Wrap you've already taken out of the Result.

On a Result with a `std_error` Wrap, use `.er_save_wrap(..)` to save a report on the inner tree. Plain `.er_save(..).er_split()` on that Result puts the Wrap itself in the pair.

The aggregation macros keep the tree when you pass them a `std_error` Wrap or its Result.

This needs your actual Wrap back. If another lib erased it into `Box<dyn Error>`, downcast it first.
