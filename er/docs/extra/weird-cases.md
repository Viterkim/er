# Weird cases

## A sub error needs Send + Sync in normal er

Your error can hold an `Rc` while it's on top. If you add another error above it, the old one becomes a sub error and needs `Send + Sync + 'static`, with the crate `er-unsync` only `'static` remains.

## When the same message prints twice

If an error puts its source's message in its own text AND returns it from `source()`, the report can print that message two times. Er prints the error, then follows `source()`. [Rust's guidance on sources](https://doc.rust-lang.org/std/error/trait.Error.html#error-source).

## Wrap with std_error

If a foreign trait needs your Wrap to implement `Error`, you can use `std_error`. Then `.er()` and `.er_with()` box the Wrap and `.er_find()` can't see inside it. Use `.er_wrap()` when the Wrap comes back, or `.er_with_wrap()` if you need the old top. [The example](../macros.md#wrap-with-std_error) shows it.
