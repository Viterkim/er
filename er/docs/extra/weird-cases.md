# Weird cases

## A sub error needs Send + Sync

Your error can hold an `Rc` while it's on top. If you add another error above it, the old one becomes a sub error and needs `Send + Sync + 'static`.

## When the same message prints twice

If an error puts its source's message in its own text AND returns it from `source()`, the report can print that message two times. Er prints the error, then follows `source()`. [Rust's guidance on sources](https://doc.rust-lang.org/std/error/trait.Error.html#error-source).

## Wrap with std_error

If a foreign trait needs your Wrap to implement `Error`, you can use `std_error`. Then normal `.er()` boxes the Wrap and `.er_find()` can't see inside it. Use `.er_wrap()` when the Wrap comes back. [The example](../macros.md#wrap-with-std_error) shows it.
