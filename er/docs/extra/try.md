# Rust 'Try'

Er is normal Result: `Er<T, E> = Result<T, ErTree<E>>`.

You might think that it would be cool if `?` would add context automatically

```rust,ignore
// Today
read_port("85").er(())?;

// Still adds stuff
read_port("67")?;
```

Needs our own result type, can't do it on the alias. Try is still nightly. HOWEVER...

Thinking about this more im not sure it would be beneficial because changing a function from returning A to B, then fixing the places that used A, is annoying. I'd rather give each function its own small `NameErr` from the start instead of exposing the inner type. And if you say "Try can do that as well" then yes ofcourse it can, but it becomes very very easy to then not add a new error to that spot, instead using the inner one which then gets error variants/cases it has no way of actually doing, and then you end up with the thiserror 'mega error enums' that have nothing to do with the scope they are in.

And `?` doesn't give `ReadFileErr` its path for free. You still have to tell it where the path comes from. With `.er(|| path)` that's tiny, and changing the error later is easy (which i think is underrated).

[Try tracking](https://github.com/rust-lang/rust/issues/84277)

[2026 goal](https://goals.rust-lang.org/2026/stabilize-try.html).
