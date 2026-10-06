# Performance

This is BASICALLY NOT RELEVANT! This is only if you really really really care, and are really interested. In most cases you will never ever hit this (also for other error handling libraries).

These comparisons are also not equal in many ways, most of the crates here don't make a tree/allow many of the functionality er/exn/rootcause/error-stack does, it's still fun to look at.

## Hot loops

If errors are rare, just use Er. With eight context layers and one failure in 10,000 items, Er and the report libraries took about 6 ns per item. thiserror and SNAFU were around 1.5 ns.

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

## Adding context

Every call fails and the error has a u32 in it.

### Just the error

```text
SNAFU            0.6 ns     0 allocations
thiserror        0.6 ns     0 allocations
Er               1.3 ns     0 allocations
Anyhow            13 ns     1 allocation
Eros              16 ns     1 allocation
Exn               17 ns     2 allocations
rootcause         26 ns     3 allocations
error-stack       35 ns     5 allocations
```

### One error around it

```text
SNAFU            0.8 ns     0 allocations
thiserror        0.8 ns     0 allocations
Eros              21 ns     3 allocations
Anyhow            24 ns     2 allocations
Er                26 ns     2 allocations
Exn               34 ns     5 allocations
error-stack       45 ns     9 allocations
rootcause         55 ns     7 allocations
```

thiserror and SNAFU keep the source inline, 'er' keeps the old error and moves its tree underneath (box for the error, vec for the sub errors). 

## Eight layers

Eight functions add their own context and half fail, time per item including the oks, allocations per failure.

### Empty structs

```text
thiserror        0.6 ns      0 allocations
SNAFU            0.6 ns      0 allocations
Eros              37 ns     3 allocations
Anyhow            61 ns     9 allocations
Exn               78 ns    17 allocations
Er                84 ns     8 allocations
error-stack       86 ns    28 allocations
rootcause        177 ns    35 allocations
```

### A u32 field

```text
thiserror        1.5 ns      0 allocations
SNAFU            1.5 ns      0 allocations
Eros              49 ns    11 allocations
Anyhow            63 ns     9 allocations
Er                93 ns    16 allocations
Exn              100 ns    26 allocations
error-stack      114 ns    37 allocations
rootcause        176 ns    35 allocations
```

### Static text

```text
thiserror        2.5 ns      0 allocations
SNAFU            2.5 ns      0 allocations
Eros              34 ns     3 allocations
Anyhow            63 ns     9 allocations
Er                83 ns    16 allocations
Exn               99 ns    26 allocations
error-stack      114 ns    37 allocations
rootcause        178 ns    35 allocations
```

### Formatted Strings

```text
Eros              85 ns    12 allocations
SNAFU             92 ns     9 allocations
thiserror        101 ns     9 allocations
Anyhow           120 ns    18 allocations
Er               154 ns    25 allocations
Exn              155 ns    35 allocations
error-stack      176 ns    46 allocations
rootcause        249 ns    44 allocations
```

## Hundred empty errors

One `TopErr(pub u32)` with a hundred unit errors under:

```text
thiserror / SNAFU Vec       about 1 ns       0 allocations
Er, known count               550 ns       1 allocation
Er, er_all                   1050 ns       6 allocations
Exn                          1190 ns     108 allocations
rootcause                    4250 ns     304 allocations
error-stack                  4840 ns     411 allocations
```

## Unsafe

### No unsafe

Er, thiserror, SNAFU and Exn don't use unsafe.

### Uses unsafe

Anyhow uses unsafe to keep its erased error as one pointer (custom vtable).

Eros uses it to move real errors back out of its erased union when narrowing it or making an enum.

Rootcause uses a lot for its erased Arc report, attachments and switching between typed and dynamic views.

Error-stack only has a few small casts and pinned Future helpers.
