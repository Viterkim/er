# er-unsync

Use `er-unsync` when errors below your top need to hold `Rc` or `RefCell`. Its `ErShared` and `ErSharedTree` use `Rc`, so you can use them on targets without pointer atomics.

```toml
er = { package = "er-unsync", version = "0.9" }
```

Keep the name `er` on the left so the derives can find it. If you call it `er_unsync` instead, each derive needs `#[er(crate = er_unsync)]`.

## Links

[GitHub Repo](https://github.com/Viterkim/er)

[Er usage and examples](https://docs.rs/er/0.9.0/er/)

[er-unsync on crates.io](https://crates.io/crates/er-unsync)

[Crates.io for the macros](https://crates.io/crates/er-macros/)

[er-unsync on Lib.rs](https://lib.rs/crates/er-unsync)
