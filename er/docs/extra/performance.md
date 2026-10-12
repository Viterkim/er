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
SNAFU             0.6 ns     0 allocations
thiserror         0.6 ns     0 allocations
Er                1.2 ns     0 allocations
Anyhow             13 ns     1 allocation
Exn                16 ns     2 allocations
Eros               16 ns     1 allocation
rootcause          26 ns     3 allocations
error-stack        34 ns     5 allocations
```

### One error around it

```text
SNAFU             0.8 ns     0 allocations
thiserror         0.8 ns     0 allocations
Eros               21 ns     3 allocations
Anyhow             23 ns     2 allocations
Er                 24 ns     2 allocations
Exn                34 ns     5 allocations
error-stack        43 ns     9 allocations
rootcause          54 ns     7 allocations
```

thiserror and SNAFU keep the source inline, 'er' keeps the old error and moves its tree underneath (box for the error, vec for the sub errors). 

## Eight layers

Eight functions add their own context and half fail, time per item including the oks, allocations per failure.

### Empty structs

```text
SNAFU             0.5 ns     0 allocations
thiserror         0.6 ns     0 allocations
Eros               40 ns     3 allocations
Anyhow             60 ns     9 allocations
Er                 74 ns     8 allocations
Exn                79 ns    17 allocations
error-stack        80 ns    28 allocations
rootcause         179 ns    35 allocations
```

### A u32 field

```text
SNAFU             1.4 ns     0 allocations
thiserror         1.5 ns     0 allocations
Eros               50 ns    11 allocations
Anyhow             61 ns     9 allocations
Er                 79 ns    16 allocations
Exn                92 ns    26 allocations
error-stack       133 ns    37 allocations
rootcause         178 ns    35 allocations
```

### Static text

```text
SNAFU             2.6 ns     0 allocations
thiserror         2.6 ns     0 allocations
Eros               39 ns     3 allocations
Anyhow             62 ns     9 allocations
Er                 82 ns    16 allocations
Exn                93 ns    26 allocations
error-stack       133 ns    37 allocations
rootcause         178 ns    35 allocations
```

### Formatted Strings

```text
Eros               88 ns    12 allocations
SNAFU              91 ns     9 allocations
thiserror          94 ns     9 allocations
Anyhow            118 ns    18 allocations
Exn               148 ns    35 allocations
Er                150 ns    25 allocations
error-stack       202 ns    46 allocations
rootcause         242 ns    44 allocations
```

## Hundred empty errors

One `TopErr(pub u32)` with a hundred unit errors under:

```text
thiserror / SNAFU Vec       0.6 ns      0 allocations
Er, known count             340 ns      1 allocation
Er, er_all                  430 ns      6 allocations
Exn                        1090 ns    108 allocations
rootcause                  4040 ns    304 allocations
error-stack                4320 ns    411 allocations
```

## Unsafe

### No unsafe

Er, thiserror, SNAFU and Exn don't use unsafe.

### Uses unsafe

Anyhow uses unsafe to keep its erased error as one pointer (custom vtable).

Eros uses it to move real errors back out of its erased union when narrowing it or making an enum.

Rootcause uses a lot for its erased Arc report, attachments and switching between typed and dynamic views.

Error-stack only has a few small casts and pinned Future helpers.
