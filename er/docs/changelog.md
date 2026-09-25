# 0.3.0

`Er<T, E>` renamed to `ErResult<T, E>`.

Rename extension traits, now ends in `Ext`, like `ErResultExt` and `ErErrorExt`.

Add the macros `er_bail!()`, `er_bail_if!()` and `er_bail_unless!()` (needs the `macros` default feature).

Add the `stack_traces` feature with optional stack traces with `.er_trace()`, they follow the tree and you have to print them yourself.

Add `.er_at_id()` / `.er_at_path()` for finding the error via an index/path in the tree.

IntoErNode is now IntoErPart, so moving a subtree keeps its traces too.

Feature gate er_all!() behind the 'macros' feature.

Fix line numbers on non Er errors in er_all!().

# 0.2.1

Cleanup / simplify the docs (especially readme.md).

# 0.2.0

Struct errors can now also use `.er(())`, `.er(|_| path)` or `.er(|_| (machine, token))` without repeating the error name.

Reports now only print the source location the first time if it's repeated.

`TestEr` is now `ErTest` and now uses the regular `.er(())`. (still uses the `test` feature).

Add option for ErLazy (I don't recommend using it, enabled with the `lazy` feature).

`.er_from_wrap(...)` is now `.er_wrap(...)`.

`er_find()` chains no longer allocate while searching (and inlined some iter stuff).

`er_find_all()` uses a faster lazy search.

Docs: Recommending `Err` for error names like `NameErr` instead of `NameEr`. And recommend using `NameError` for the public external non Er types. And `let e = ||` for closures.

Docs: `tricky-error-comparison.md` docs added (foreign errors, own errors, the original error, string context, typed context, using / consuming, public boundary).

Docs: `performance.md`, niche little thing.

# 0.1.3

Readme and docs changes.

Optional serde for snapshots.

Add `.er_with(|t|)` / `.er_with(|e|)` to copy fields off the previous error, without `map_err` accidentally destroying the tree.

# 0.1.2

Accidentally pushed stuff with wrong readme. And you can't undo it great.

# 0.1.1

Initial release (yes, the 'Er' name was taken by a spam crate, and i accidentally pushed an older `er-macros` `0.1.0` early).
