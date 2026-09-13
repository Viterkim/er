# er-macros

Derives for [Er](https://docs.rs/er/latest/er/).

If you want `.er(...)` and the tree, then use `er`. 

If you only want the macros like `#[derive(Er)]` and `#[derive(ErFormat)]` for stuff like Display, Debug, constructors, then you can just use this. 

```rust
use er_macros::{Er, ErFormat};

#[derive(ErFormat)]
struct File {
    name: String,
}

#[derive(Er)]
struct ReadEr {
    file: File,
}

let error = ReadEr::new(File::new("missing.txt"));
println!("{error}");
```

```text
ReadEr { file: File { name: "missing.txt" } }
```

More options: [macros.md](https://github.com/Viterkim/er/blob/main/er/docs/macros.md) (`format`, `skip`, `censor`, `wrap`, etc).
