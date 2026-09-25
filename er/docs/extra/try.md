# Rust 'Try'

Er is normal Result: `ErResult<T, E> = Result<T, ErTree<E>>`.

You might think that it would be cool if `?` would add context automatically

```rust,ignore
// Today
read_port("85").er(())?;

// Still adds stuff
read_port("67")?;
```

Needs our own result type, can't do it on the alias. (so we would have to make Er be its own type so not an alias to a Result).

It would probably only help for .er(()) and make a raw '?' work, but im not sure it would actually work, and i don't know if its coming out.

[Try tracking](https://github.com/rust-lang/rust/issues/84277)

[2026 goal](https://goals.rust-lang.org/2026/stabilize-try.html).
