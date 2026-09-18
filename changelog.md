# 0.2.0

Struct errors can now also use `.er(())`, `.er(|| path)` or `.er(|| (machine, token))` without repeating the error name.

TestEr -> ErTest and now uses the regular `.er(())`.

Docs and readme changes (recommending `Err` for error names like `NameErr` instead of `NameEr`. And recommend using `NameError` for the public external non Er types).

Tricky-error-comparison.md docs added (foreign errors, own errors, the original error, string context, typed context, using / consuming, public boundary).

TODO: maybe a lazy variant

TODO: maybe a more convenient enum varint thing (maybe type E = VeryLongErrorName; thing.er(|| E::invalid_input(input))?;) but probably not

TODO: Make normal .er() work with a Wrap that has std_error. Right now it boxes the Wrap, so .er_find() sees the Wrap but can't see the errors inside it. .er_from_wrap() works, but having to remember a different method sucks. We tried letting source() point back to the tree. Other libraries printed an extra cause, so we dropped it. Maybe Er can spot its own Wrap before boxing it. We haven't found a clean way to do that yet.


# 0.1.3

Readme and docs changes.

Optional serde for snapshots.

Add `.er_with(|t|)` / `.er_with(|e|)` to copy fields off the previous error, without `map_err` accidentally destroying the tree.

# 0.1.2

Accidentally pushed stuff with wrong readme. And you can't undo it great.

# 0.1.1

Initial release (yes, the 'Er' name was taken by a spam crate, and i accidentally pushed an older `er-macros` `0.1.0` early).
