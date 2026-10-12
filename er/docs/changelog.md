# 0.8.0

Removed `.opaque_err()` and `ErAsError`. Instead export your saved diagnostics with `.er_into()` (or use an explicit Wrap to keep a live tree on the external trait cases).

For your error types where you maybe don't use the tree, you can now use `#[er(src_location)]` which captures a location on your type instead of on the tree (and leaves the field out of constructor args). You can use it above a `pub some_name: ErSrcLocation` field (which is just an alias for `&'static core::panic::Location<'static>`).

Added `#[er(display = "...")]` and `#[er(debug = "...")]` which override each output, `format` still does both. Also now `display_alt` for `{:#}` (each can also take a formatting functions).

`#[er(default)]` lets tuple helpers omit fields. Use `#[er(default = make_data())]` for your own fallback. Supplying those fields still works.

`.er_val()` now takes the same field and tuple helpers as `.er()`, like `.er_val(|v| (path, v))` (for old finished struct there's `er_built(...)`, same as `.er_with()`, but just use the new pattern like on `.er()`).
Replace `.er_val(DeviceErr::new)` with `.er_val(|v| v)` for a single field.

`.er_into()` now gets the owned report and uses those helpers too, like `.er_into(|report| report.er_report_string())`. You can build another library's error without caring about 'er' or the version.

Automatic report saving now uses ordinary field conversions. `ErFromTree` is gone, use `.er_into()` to finish the conversion.

For automatic `into_*` filling, change `.er_into(|_| ())` to `.er_into(|report| report)`. With extra fields, change `.er_into(|_| fields)` to `.er_into(|report| (report, fields))`.

Snapshot Results get the printing and unwrap helpers. `ErSnapshotReport` and `ErSnapshotTop` now own the snapshot, rename the old borrowed types to `ErSnapshotReportRef` and `ErSnapshotTopRef`. Snapshot report/top views no longer implement `Error`. Save their diagnostics in an ordinary error when a foreign API requires one.

`.er_test()?` now also handles `Result<T, ()>`.

# 0.7.0

`.er_with()` takes fields now, example: `.er_with(|err| (err.code, path))`. Use `er_built(...)` if you assemble the whole error yourself. Same for the tree and Wrap forms.

Add the `er-unsync` crate which is the same api with Rc sharing and stored errors without Send/Sync requirements.

`.er_into()` can fill extra fields, like `.er_into(|_| (request_id, path))`.

# 0.6.0

Add `.er_into(|tree| ...)` for converting to public errors with a report or snapshot.

Add `.er_report_string()` / `.er_top_string()` and `.er_snapshot()` on Results.

Add `.er_test()` for Options and Results.

Make `er_bail!()` take the tuple/convenience constructors.

Accept bare errors and trees in `er_all!()` / `er_add!()` collections.

Fix helpers on Wrap and ErTest.

# 0.5.3

Add optional #[er(source)] for public boundary errors (not the reports/trees).

# 0.5.2

Make docs more explicit++ (after feedback).

# 0.5.1

Make docs more explicit.

# 0.5.0

Added `er_try!(top, [results])`, like `er_all!(top, [results])` but it gives you the Ok values back if they all succeed.

Enabled the `test` feature by default.

`ErTest` now does `?` without `.er()` (but a failure from another `ErTest` passes through without adding context, so use a specific type for your test helpers).

For `ErTest` renamed the `TestError` type to `ErTestError` so `use er::*` doesn't take a potential user error name in tests. It still prints as `TestError`.

`.unwrap_report()` and `.expect_report(message)` added.

# 0.4.0

New `.er_add([error])` instead of `with_part` and `push_part`.

`er_add!(tree, [first, second])` added (mixed types and results behind the `macros` feature).

`.er_with(|err|)` now gets the 'top' error instead of the tree. So `err.top.path` is now `err.path` (If you need the old one, you can use `.er_with_tree(|tree|)`).

Add `ErShared<T>` for sharing data between errors by passing `&err.field` (used with `.er_with(|err|)`).

# 0.3.0

`Er<T, E>` renamed to `ErResult<T, E>`.

Rename extension traits, now ends in `Ext`, like `ErResultExt` and `ErErrorExt`.

Add the macro `er_bail!()` (needs the `macros` default feature).

Add the `stack_traces` feature with optional stack traces with `.er_trace()`, they follow the tree and you have to print them yourself.

Add `.er_at_index()` / `.er_at_path()` for finding the error via an index/path in the tree.

IntoErNode is now IntoErPart, so moving a subtree keeps its traces too.

For iter added `.er_collect()` for collecting and stopping on the first error, or `.er_collect_all()` which keeps going and adds all as sub errors.

`er_all!` now supports errors, results and trees (also mixed), and its now behind the 'macros' feature.

Fix line numbers on non Er errors in er_all!().

Raw errors take `.er(context)` like results and trees.

`.er_wrap()` now supports the construction helpers.

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

Add `.er_with(|tree|)` / `.er_with(|err|)` to copy fields off the previous error, without `map_err` accidentally destroying the tree.

# 0.1.2

Accidentally pushed stuff with wrong readme. And you can't undo it great.

# 0.1.1

Initial release (yes, the 'Er' name was taken by a spam crate, and i accidentally pushed an older `er-macros` `0.1.0` early).
