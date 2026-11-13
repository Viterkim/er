# Good and bad

A few things that look fine until you have to use the error. For the patterns agents keep generating, see [What agents miss](https://github.com/Viterkim/er/blob/main/er/docs/extra/what-agents-miss.md).

## Messages are fine, data goes in fields

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

The port is right there in the output already. Gluing it into the message buys you nothing, and now your caller has to parse `85` back out of a string. Keep the message static and put the values next to it, reaching for `format!` in an error means you're missing a field.

## Add what you know, don't pass it down

```rust
// Bad, the path only goes down so the inner error can carry it
pub fn pending_count(store: &Store) -> ErResult<i64, PendingCountErr> {
    count_rows(&store.db, store.path().to_path_buf()).er(store.path())
}
pub fn count_rows(db: &Connection, path: PathBuf) -> ErResult<i64, CountRowsErr> {
    db.query_row(COUNT, [], |row| row.get(0)).er(|_| path)
}

// Good
pub fn pending_count(store: &Store) -> ErResult<i64, PendingCountErr> {
    count_rows(&store.db).er(store.path())
}
pub fn count_rows(db: &Connection) -> ErResult<i64, CountRowsErr> {
    db.query_row(COUNT, [], |row| row.get(0)).er(())
}
```

Every layer adds what it knows, and the caller already knows the path. Pass it down too and the report prints it on every level, and the inner function grows a parameter that does nothing but feed an error.

## NEVER put a tree in a field

```rust
// Bad
#[derive(Clone, Er)]
#[er(format = Self::report)]
pub struct ProtectionFailure {
    #[er(skip)]
    cause: ErShared<ErTree<ProtectErr>>,
}

// Good
pub protected: Result<(), ErSharedTree<ProtectErr>>,

self.protected.as_ref().er(|| SendErr::protection(id))?;
```

Errors go in the tree. A tree hiding in a field doesn't show up in the report and `er_find` can't see it, so you end up writing your own report function just to print it, that's the `format = Self::report` up there. When a bunch of places need the same failure, `.er_share()` it and put it below your error like anything else. The derive warns you about this one.

## Save reports where they arrive

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

A report a library handed you as a String is gone from every print that doesn't go through your glue, and that's most of them. `.er_save()` it where it comes in, while their error is still in your hand. A library that gives you `(error, report)` already did it for you, a plain `.er()` saves it.

It's for whole reports from somewhere else. `"user id 85"` would print fine in there too, but it belongs on your type where someone can match on it.

## UI messages keep the type

```rust
// Bad
Saved(Result<(), String>),

Task::perform(save(path), |result| Message::Saved(result.er_report()))

// Good
Saved(Result<(), ErSharedTree<SaveErr>>),

Task::perform(save(path), |result| Message::Saved(result.er_share()))
```

When iced needs to clone a message, share the tree. In `update()` you can still match on `error.top` and decide what to show. A String would leave you with the text before you even got to that decision.

With iced's `debug` feature the message also needs `Debug`. Derive `Clone` for `SaveErr` and keep it next to its report.

```rust
Saved(Result<(), (SaveErr, String)>),

Task::perform(save(path), |result| Message::Saved(result.er_split()))
```

You still get the error in `update()`, and a plain `.er()` keeps the pair when another operation fails because of it.

## er_find is for logs and tests

```rust
// Bad, somewhere way up
if error.er_find::<OwnerErr>().is_some_and(|err| matches!(err, OwnerErr::Capacity)) {
    return retry_later();
}

// Good, one layer up, then your caller just matches SourceErr::Capacity
publish(bytes).er_with(|err| match err {
    OwnerErr::Capacity | OwnerErr::Exhausted => SourceErr::Capacity,
    _ => SourceErr::failed(bytes),
})?;
```

Never branch your logic on it. Nothing connects that `er_find` to the code making the error, so when someone moves `Capacity` into another type next month your `if` quietly turns into "never". Logic that cares about it wants it in your own type. (And `er_find().is_some()` is just `.er_contains()`.)

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

`is_err()` passes no matter why it failed, so it keeps passing when your code breaks for a completely different reason. The `.contains("revoked")` one breaks the second someone rewords a message. And `is_ok()` just tells you `false`, while `?` prints the whole report so you see what happened, and `er_assert_err!` prints it too when it gets Ok or the wrong error.
