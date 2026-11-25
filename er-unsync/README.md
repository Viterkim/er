# er-unsync

Single threaded / unsync version of `er` for platforms that don't use Arc, and want to use Rc/RefCell etc.

```toml
er = { package = "er-unsync", version = "0.9" }
```

You need the name `er` on the left so the derives can find it. If you call it `er_unsync` instead, each derive needs `#[er(crate = er_unsync)]`.

## Links

[Examples / Patterns](https://github.com/Viterkim/er/blob/main/er/docs/examples.md)

[Good and bad](https://github.com/Viterkim/er/blob/main/er/docs/good-and-bad.md)

[GitHub Repo](https://github.com/Viterkim/er)

[Er usage and examples](https://docs.rs/er/0.9.0/er/)

[er-unsync on crates.io](https://crates.io/crates/er-unsync)

[Crates.io for the macros](https://crates.io/crates/er-macros/)

[er-unsync on Lib.rs](https://lib.rs/crates/er-unsync)
