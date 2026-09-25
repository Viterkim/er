# Performance

This is BASICALLY NOT RELEVANT! This is only if you really really really care, and are really interested. In most cases you will never ever hit this (also for other error handling libraries).

## Hot loops

If errors are rare, just use Er. With one failure in 10,000 items, Er took about 1.24 ns per item(Most of the libs are around the same).

If half the items are expected to fail, make normal errors in the loop and aggregate them once after.

```rust
use er::*;

#[derive(Er)]
pub struct ItemErr(pub usize);

#[derive(Er)]
pub struct CheckAllErr;

fn check_all(results: impl Iterator<Item = Result<(), ItemErr>>) -> ErResult<(), CheckAllErr> {
    er_all!((), results)
}
```

## Failed layers

Each failed `.er()` keeps the old error and moves its old tree underneath (no tree rebuilding).

A layer with data usually needs one allocation for the old error and one for its sub errors. Unit/empty errors are cheaper (boxing a zero sized value does not allocate).

Eight successful layers allocate nothing. Eight failed layers above one foreign error used 9 allocations with unit errors, 16 with typed errors or `&'static str`, and 24 with lazy owned strings.

## Lots of errors

Tiny custom error, then 8 functions add their own error on top, and 50% fail.

```text
               per item       per failure
thiserror      14.8 ns     0 allocations
SNAFU          14.8 ns     0 allocations
Eros           42.9 ns    10 allocations and 1 reallocation
Anyhow         50.8 ns     8 allocations
Er             62.7 ns    16 allocations (thats us, wow)
Problemo       69.5 ns     2 allocations and 2 reallocations
Exn            84.5 ns    26 allocations
error-stack    89.1 ns    37 allocations
rootcause     172.2 ns    35 allocations
```

The typed errors and SNAFU keep their source chain in the type. Er and the report libraries build history when something fails.

Aggregating failures took about `156 ns` per item for Er, 40 ns for thiserror and 25 ns for SNAFU.

Used similar kind of context as the [tricky comparison](../tricky-error-comparison.md).

## Unsafe

### No unsafe

Er, thiserror, SNAFU and Exn don't use unsafe.

Problemo also doesn't but it uses `ThinVec` which does.

### Uses unsafe

Anyhow uses unsafe to keep its erased error as one pointer (custom vtable).

Eros uses it to move real errors back out of its erased union when narrowing it or making an enum.

Rootcause uses a lot for its erased Arc report, attachments and switching between typed and dynamic views.

Error-stack only has a few small casts and pinned Future helpers.
