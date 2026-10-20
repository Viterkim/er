# Performance

This is BASICALLY NOT RELEVANT! This is only if you really really really care, and are really interested. In most cases you will never ever hit this (also for other error handling libraries).

These comparisons are also not equal in many ways, most of the crates here don't make a tree/allow many of the functionality er/exn/rootcause/error-stack does, it's still fun to look at.

## Hot loops

If errors are rare, just use Er. With eight context layers and one failure in 10,000 items, Er, error-stack and rootcause took about 6 ns per item, Eros about 7 ns. Anyhow and Exn were around 1 ns, thiserror and SNAFU around 1.5 ns.

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
Er                1.4 ns     0 allocations
Eros              8.5 ns     1 allocation
Anyhow             13 ns     1 allocation
Exn                15 ns     2 allocations
rootcause          18 ns     3 allocations
error-stack        24 ns     5 allocations
```

### One error around it

```text
SNAFU             0.8 ns     0 allocations
thiserror         0.8 ns     0 allocations
Eros               20 ns     3 allocations
Er                 22 ns     2 allocations
Anyhow             22 ns     2 allocations
Exn                32 ns     5 allocations
error-stack        41 ns     9 allocations
rootcause          47 ns     7 allocations
```

thiserror and SNAFU keep the source inline, 'er' keeps the old error and moves its tree underneath (box for the error, vec for the sub errors). 

## Just creating Er errors

Keeping 128 failed results alive at a time, with dropping outside the timer:

```text
One u32 context            18 ns
Eight u32 contexts        121 ns
Eight empty contexts       95 ns
```

`.er(|_| code)` and `.er(|| MyErr { code })` both took about 18 ns for one context.

## Eight layers

Eight functions add their own context above the original error and half the calls fail, time per item including the oks, allocations per failure.

### Empty structs

```text
SNAFU             0.6 ns     0 allocations
thiserror         0.6 ns     0 allocations
Eros               33 ns     3 allocations
Anyhow             52 ns     9 allocations
Er                 72 ns     8 allocations
Exn                77 ns    17 allocations
error-stack        79 ns    28 allocations
rootcause         168 ns    35 allocations
```

### A u32 field

```text
SNAFU             1.4 ns     0 allocations
thiserror         1.4 ns     0 allocations
Eros               44 ns    11 allocations
Anyhow             48 ns     9 allocations
Er                 80 ns    16 allocations
Exn                86 ns    26 allocations
error-stack       104 ns    37 allocations
rootcause         169 ns    35 allocations
```

### Static text

```text
SNAFU             2.4 ns     0 allocations
thiserror         2.4 ns     0 allocations
Eros               32 ns     3 allocations
Anyhow             59 ns     9 allocations
Er                 81 ns    16 allocations
Exn                94 ns    26 allocations
error-stack       103 ns    37 allocations
rootcause         172 ns    35 allocations
```

### Formatted Strings

```text
Eros               81 ns    12 allocations
SNAFU              83 ns     9 allocations
thiserror          94 ns     9 allocations
Anyhow            109 ns    18 allocations
Er                140 ns    25 allocations
Exn               149 ns    35 allocations
error-stack       217 ns    46 allocations
rootcause         230 ns    44 allocations
```

## Hundred empty errors

One `TopErr(pub u32)` with a hundred unit errors under:

```text
thiserror / SNAFU Vec       1.0 ns      0 allocations
Er, known count             320 ns      1 allocation
Er, er_all                  678 ns      6 allocations
Exn                         993 ns    108 allocations
rootcause                  3866 ns    304 allocations
error-stack                4066 ns    411 allocations
```

## Unsafe

### No unsafe

Er, thiserror, SNAFU and Exn don't use unsafe.

### Uses unsafe

Anyhow uses unsafe to keep its erased error as one pointer (custom vtable).

Eros uses it to move real errors back out of its erased union when narrowing it or making an enum.

Rootcause uses a lot for its erased Arc report, attachments and switching between typed and dynamic views.

Error-stack only has a few small casts and pinned Future helpers.

## Versions

Er 0.8.1, thiserror 2.0.20, SNAFU 0.9.2, Anyhow 1.0.104, Exn 0.3.1, Eros 0.8.0, rootcause 0.13.0, error-stack 0.8.0.
