# Macros

Check out the main examples.md doc for patterns to use, this is mostly just for the macros specifically.

## Constructors

Both `Er` and `ErFormat` give structs a `new` and enum variants a method named after the variant.

```rust,ignore
#[derive(Er)]
pub struct FindErr {
    pub query: String,
    pub account: u64,
}

FindErr::new("settings", 85);

#[derive(Er)]
pub enum ModeErr {
    Missing,
    Unknown { input: String },
}

ModeErr::missing();
ModeErr::unknown("turbo");
```

The arguments follow field order. Here `"settings"` becomes a `String`, and `85` gets its type from `account`. Put `#[er(exact)]` on a field to take its type directly.

With `Er`, you can use those same fields in `.er(("settings", 85))` and let the return type pick `FindErr`. Put `#[er(default)]` on trailing fields you want to leave out. Er fills each missing field with `Default::default()`, or your expression if you wrote `#[er(default = make_data())]`. `new` still takes those fields.

`#[er(src_location)]` fills a `Location` for you, even with the `src_locations` feature off. Use `.er(|| LocErr::new())` to capture the line where you made the call.

The constructors have the same visibility as the type. On a `pub struct`, a `pub(crate)` field keeps `new` inside the crate, and a private field keeps it inside the module. Use `.er(|| MyErr::new(...))` there. Mixing absolute and relative field scopes can keep `new` inside the module too. The tuple form would let anyone construct the error because Rust trait impls are public.

Er skips constructors on a `#[non_exhaustive]` struct or variant. On a `#[non_exhaustive]` enum, its existing variants still get their constructors unless you mark the variants too. Use `#[er(no_constructors)]` when you want to write your own `new` or turn off field construction altogether, including `.er(())`. You can also put it on a single variant like `Failed(Infallible)` to skip just that constructor.

The derives call er's own items through `::er`, so a renamed er needs `#[er(crate = other_name)]`.

## Wrap

Use Wrap to implement another crate's trait on your error tree. You can already print the tree with `.er_report()` and share it between owners with `.er_share()`.

Rust needs you to own the type or the trait. Wrap gives you a type you *do* own around `ErTree`. [Orphan rule](https://doc.rust-lang.org/reference/items/implementations.html#trait-implementation-coherence)

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

If the foreign API needs Display/Debug on the Wrap itself, add `output = report` or `output = top` (Axum doesn't care).

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

If you need the old top, you can use `.er_with_wrap(|err| fields)`. To add the Wrap as a child, use `report.er_add_wrap(wrapped)` on a tree or branch. In a mixed list, `er_add!(report, [wrapped, other])` keeps the wrapped tree for you.

This needs your actual Wrap back. If another lib boxed it, downcast it first.
