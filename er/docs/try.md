# Rust 'Try'

Er is normal Result: `Er<T, E> = Result<T, ErTree<E>>`.

You might think that it would be cool if `?` would add context automatically 

```rust,ignore
// Today
read_port("85").er(ReadEr::new)?;

// Still adds stuff 
read_port("67")?;
```

Needs our own result type, can't do it on the alias. Try is still nightly. HOWEVER...

Thinking about this more im not sure it would be beneficial because changing a function from returning A to B, then fixing the places that used A, is annoying. I'd rather give each function its own small ErType from the start instead of exposing the inner type. And if you say "Try can do that as well" then yes ofcourse it can, but it become very very easy to then not add a new error to that spot, instead using the inner one which then gets error variants/cases it has no way of actualy doing, and then you end up with the thiserror 'mega error enums' that have nothing to do with the scope they are in.

Also you still gotta supply values like `ReadFileEr::new(path)`, Try can't guess, and that means more friction when stuff changes (and i believe that having stuff be easy to change over time, is an underrated benefit).

[Try tracking](https://github.com/rust-lang/rust/issues/84277), [2026 goal](https://goals.rust-lang.org/2026/stabilize-try.html).
