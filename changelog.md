# 0.2.0

Struct errors can now also use `.er(())`, `.er(|| path)` or `.er(|| (machine, token))` without repeating the error name.

Reports now only print the source location the first time if it's repeated.

TestEr is now ErTest and now uses the regular `.er(())`. (still used the `test` feature).

Added option for ErLazy(I don't recommend using it, enabled with the `lazy` feature).

`.er_from_wrap(...)` is now `.er_wrap(...)`.

Docs: Recommending `Err` for error names like `NameErr` instead of `NameEr`. And recommend using `NameError` for the public external non Er types.

Docs: `Tricky-error-comparison.md` docs added (foreign errors, own errors, the original error, string context, typed context, using / consuming, public boundary).

# 0.1.3

Readme and docs changes.

Optional serde for snapshots.

Add `.er_with(|t|)` / `.er_with(|e|)` to copy fields off the previous error, without `map_err` accidentally destroying the tree.

# 0.1.2

Accidentally pushed stuff with wrong readme. And you can't undo it great.

# 0.1.1

Initial release (yes, the 'Er' name was taken by a spam crate, and i accidentally pushed an older `er-macros` `0.1.0` early).
