# Weird cases

## When .er() has two choices

The boxed field takes any error. When `existing` is already a `BoxErr`, `.er(|| existing)` could use it as the top error or put it in the box of a new one. Rust refuses to guess and gives a compiler error. An `io::Error` only fits the box, so that works normally.

```rust
#[derive(Er)]
pub struct BoxErr(pub Box<dyn std::error::Error + Send + Sync>);

// Use the BoxErr we already made
pub fn use_existing(result: Result<(), io::Error>, existing: BoxErr) -> Er<(), BoxErr> {
    ErContext::<ErBuilt>::er(result, || existing)
}

// Put it inside a new BoxErr
pub fn wrap_existing(result: Result<(), io::Error>, existing: BoxErr) -> Er<(), BoxErr> {
    ErContext::<ErFields>::er(result, || existing)
}
```

## A sub error needs Send + Sync

Your error can hold an `Rc` while it's on top. If you add another error above it, the old one becomes a sub error and needs `Send + Sync + 'static`.

## When the same message prints twice

If an error puts its source's message in its own text AND returns it from `source()`, the report can print that message two times. Er prints the error, then follows `source()`. [Rust's guidance on sources](https://doc.rust-lang.org/std/error/trait.Error.html#error-source).

## Wrap with std_error

If a foreign trait needs your Wrap to implement `Error`, you can use `std_error`. Then normal `.er()` boxes the Wrap and `.er_find()` can't see inside it. Use `.er_wrap()` when the Wrap comes back. This is probably the worst bit of Er right now. [The example](../macros.md#wrap-with-error) shows it.
