# Weird cases

## A sub error needs Send + Sync in normal er

An error with an `Rc` can sit on top. Adding context puts it in the tree's boxed errors, which require `Send + Sync + 'static`. With `er-unsync` you can keep the `Rc` down there too.

## When the same message prints twice

Er prints the error and then follows `source()`. When the error's own message already includes the cause, you see it twice. [Rust's guidance on sources](https://doc.rust-lang.org/std/error/trait.Error.html#error-source).

## Wrap with std_error

Use `std_error` when a foreign trait needs your Wrap to implement `Error`. On a Result from that API, `.er()` and `.er_with()` box the Wrap, so `.er_find()` can't see inside. Use `.er_wrap()` to keep the tree or `.er_with_wrap()` if you also need to read the old top. [Wrap example](../macros.md#wrap-with-std_error)
