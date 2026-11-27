# Good and bad (old habits die hard)

## We are not javascript, don't jam everything into a String

```rust
// Bad
#[derive(Er)]
pub struct PortErr {
    pub msg: String,
}

er_bail!(|_| format!("Incorrect port: {port}"));

// Good
#[derive(Er)]
pub struct PortErr {
    pub msg: &'static str,
    pub port: u16,
}

er_bail!(("Incorrect port", port));
```

```text
PortErr { msg: "Incorrect port", port: 85 } @ src/main.rs:11:9
```

The display/debug already shows it nicely, forcing it all into a string gains nothing and now your caller has to parse `85` back out of a string if they need it. Keep the message static and put the values next to it, reaching for `format!` in an error usually means you're missing a field.

## Never rebuild a tree with map_err (or drop it on bail)

```rust
// Bad, nukes the whole tree underneath
read_device().map_err(|err| er_new!(AnalyzeErr::new(err.top.code)))

// Also bad, the t tree is gone (womp womp, sad sounds)
if let Err(t) = read_device() {
    er_bail!(AnalyzeErr::new(t.top.code));
}

// Good, your error on top and the original failure still below it
read_device().er_with(|err| err.code)
```

This is the #1 mistake from other report style error handling libs, you will one day accidentally nuke the tree, avoid map_err.

`er_new!` is for starting a brand new error. If a call already gave you an error, ALWAYS add onto it with `.er()` or `.er_with()` so you keep the original error, its location and everything below it. When you're collecting failures into one report, `.er_add()` keeps the locations they came with. Put `.er(..)` at the call whose line you want to see.

## Carry a shared case/logical flow data up

```rust
// Bad, every layer spells the same match out again
pack(frame).er_with(|err| match err {
    PackErr::Capacity(_) => SendErr::capacity(peer),
    PackErr::Failed(_) => SendErr::failed(peer),
})?;

// Good, the case is a field and each layer passes it along
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Pressure {
    Capacity,
    Failed,
}

pack(frame).er_with(|err| (peer, err.pressure))?;
```

Mapping another error into yours is fine one layer up, but if the same case has to travel through five layers, just put it in a small `Copy` enum and pass it along as a field. Every layer still adds its own stuff, and only the code actually acting on it matches on it (so adding a third case later gives you one compile error).

## Stop fighting inference (and don't wrap Err again)

```rust
// Bad
let config = read_config(path).er::<LoadErr>(())?;

match load_file(path) {
    Ok(file) => file,
    Err(error) => return Err(error).er(|| LoadErr::open(&path)),
}

// Good
let config = read_config(path).er(())?;

match load_file(path) {
    Ok(file) => file,
    Err(error) => er_bail!(LoadErr::open(&path), error),
}
```

Your function already returns `ErResult<_, LoadErr>`, `::<LoadErr>` is just spam (you only need the turbofish inside `er_all!` or mid chain where Rust can't infer it yet). And when you already matched `Err(error)`, you can give both the new error and the cause to `er_bail!` (or `er_new!` if you want the tree back) instead of wrapping it back into `Err(error).er(..)`.


## NEVER put a tree in a field

```rust
// Bad
#[derive(Clone, Er)]
#[er(format = Self::report)]
pub struct OpenFailure {
    #[er(skip)]
    cause: ErShared<ErTree<OpenErr>>,
}

// Good
pub opened: Result<Speaker, ErSharedTree<OpenErr>>,
self.opened.as_ref().er(|| PlayErr::closed(id))?;
```

Errors go in the tree. A tree hiding in a field won't show up in reports and `er_find` can't see it, so you end up writing your own report function just to print it (like `format = Self::report` above). When a bunch of places need the same failure, `.er_share()` it and put it below your error like anything else (the derive even warns you). 

## Save reports at arrival

```rust
// Bad, only prints where someone remembers to call render()
pub fn render<E: Error + 'static>(error: &ErTree<E>) -> String {
    let mut text = error.er_report();
    for reply in error.er_find_all::<ServerReply>() {
        text.push_str(&reply.body);
    }
    text
}

// Good, prints with every report from now on
client.send(request).er_save("server", |err| &err.body).er(id)?;
```

If a library hands you a report string on its error, `.er_save()` it right where it comes in instead of writing some custom `render()` helper nobody remembers. (If the library returns `(error, report)`, the good old `.er()` already does this for you.)

`.er_save()` is for actual reports from somewhere else (maybe on a type you are given).

## Don't throw away your error type for a String

`ErTree` doesn't implement `Clone` or `Debug`, so when a message enum or channel needs them it seems easy to make a `Result<(), String>` with `.er_report()` and lose `SaveErr`:

```rust
// Bad, now the user only has text and can't match / do flow stuff
Saved(Result<(), String>),
save(path).er_report()

// Good (done with the tree / message and you need Debug + Clone): finish with .er_split()
Saved(Result<(), (SaveErr, String)>),
save(path).er_split()

// Good (still inside your code, needs Clone, no Debug): sharing is caring 
Saved(Result<(), ErSharedTree<SaveErr>>),
save(path).er_share()
```

## Overusing er_find

You can argue that a flaw with error reporting libs is that they at max only have the top error typed, and that is true, so a way that's easy to get around it is with searching/finding. The issue is that that's a runtime check and now your compiler can't help you if stuff changes.

```rust
// Bad, somewhere way up
if let Some(OwnerErr::Capacity) = error.er_find::<OwnerErr>() {
    return retry_later();
}

// Good, one layer up, then your caller just matches SourceErr::Capacity
publish(bytes).er_with(|err| match err {
    OwnerErr::Capacity | OwnerErr::Exhausted => SourceErr::Capacity,
    _ => SourceErr::failed(bytes),
})?;
```

## Add what you know, don't pass it down

```rust
// Bad, parse_port doesn't touch path (don't pass it down)
pub fn load_config(path: &Path) -> ErResult<u16, LoadConfigErr> {
    let text = fs::read_to_string(path).er(path)?;
    parse_port(&text, path).er(path)
}
pub fn parse_port(text: &str, path: &Path) -> ErResult<u16, ParsePortErr> {
    text.trim().parse().er((text, path))
}

// Good, each function only adds what it actually works with
pub fn load_config(path: &Path) -> ErResult<u16, LoadConfigErr> {
    let text = fs::read_to_string(path).er(path)?;
    parse_port(&text).er(path)
}
pub fn parse_port(text: &str) -> ErResult<u16, ParsePortErr> {
    text.trim().parse().er(text)
}
```

## Tests should check WHICH error

```rust
// Bad
assert!(write(&path, "x").is_err());
let error = write(&revoked_path, "x").unwrap_err();
assert!(error.er_report().contains("revoked"));
assert!(save(&path).is_ok());

// Good, in a test returning ErTest
er_assert_err!(write(&path, "x"), WriteErr::DifferentExisting(_));
let error = er_assert_err!(write(&revoked_path, "x"));
assert!(error.er_contains::<RevokedErr>());
save(&path)?;
```
