# Performance

This is BASICALLY NOT RELEVANT! Most apps spend their time doing the work that sometimes fails. It's still fun to see what an error costs, especially when some of these crates are keeping a whole tree and others just keep the cause in a field.

## Binary size

One program makes 200 error types with `er_new!` and prints their reports. Its stripped binary is 588 KiB with either Er or er-unsync.

`#[derive(Er)]` writes about 1 KB of Rust for a unit struct, and 258 KiB for an enum with a `String` field in each of its 1,000 variants.

## Hot loops

If errors are rare, just use Er. With eight context layers and one failure in 10,000 items, Er, Anyhow, Exn, error-stack and rootcause all took about 6 ns per item. Eros took about 6.5 ns, thiserror and SNAFU about 1.5 ns.

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
SNAFU           0.6 ns     0 allocations
thiserror       0.6 ns     0 allocations
Er              1.8 ns     0 allocations
Anyhow           13 ns     1 allocation
Eros             16 ns     1 allocation
Exn              21 ns     2 allocations
rootcause        26 ns     3 allocations
error-stack      36 ns     5 allocations
```

### One error around it

```text
SNAFU           0.8 ns     0 allocations
thiserror       0.8 ns     0 allocations
Eros             21 ns     3 allocations
Er               23 ns     2 allocations
Anyhow           24 ns     2 allocations
Exn              34 ns     5 allocations
error-stack      43 ns     9 allocations
rootcause        56 ns     7 allocations
```

thiserror and SNAFU keep the source inline, 'er' keeps the old error and moves its tree underneath (box for the error, vec for the sub errors).

## Just creating Er errors

Keeping 128 failed results alive at a time, with dropping outside the timer:

```text
One u32 context            19 ns
Eight u32 contexts        120 ns
Eight empty contexts      101 ns
```

`.er(code)`, `.er(|_| code)` and `.er(|| MyErr { code })` took about the same time for one context.

## Eight layers

Eight functions add their own context above the original error and half the calls fail. Time is per item, including the oks. Allocations are per failure.

### Empty structs

```text
thiserror      0.55 ns     0 allocations
SNAFU          0.55 ns     0 allocations
Eros             38 ns     3 allocations
Anyhow           61 ns     9 allocations
Er               62 ns     8 allocations
Exn              79 ns     17 allocations
error-stack      79 ns     28 allocations
rootcause       174 ns     35 allocations
```

Er allocates one list of sub errors per layer. The empty errors themselves take up no space in their boxes.

### A u32 field

```text
SNAFU           1.4 ns     0 allocations
thiserror       1.5 ns     0 allocations
Eros             50 ns     11 allocations
Anyhow           63 ns     9 allocations
Er               69 ns     16 allocations
Exn              96 ns     26 allocations
error-stack     139 ns     37 allocations
rootcause       179 ns     35 allocations
```

### Static text

```text
SNAFU           2.5 ns     0 allocations
thiserror       2.5 ns     0 allocations
Eros             40 ns     3 allocations
Anyhow           65 ns     9 allocations
Er               68 ns     16 allocations
Exn              97 ns     26 allocations
error-stack     137 ns     37 allocations
rootcause       185 ns     35 allocations
```

### Formatted Strings

```text
Eros             92 ns     12 allocations
SNAFU            93 ns     9 allocations
thiserror       101 ns     9 allocations
Anyhow          120 ns     18 allocations
Er              136 ns     25 allocations
Exn             154 ns     35 allocations
error-stack     209 ns     46 allocations
rootcause       255 ns     44 allocations
```

## Printing the report

A unit error with its source location took about 50 ns to print. Printing a `String` containing 4,200 bytes with `#[er(format = "{0}")]` took 190 ns and one allocation. The default format took 1.3 µs for the same text. Debug quotes the string and checks which characters need escaping.

## Hundred empty errors

One `TopErr(pub u32)` with a hundred unit errors underneath. `er_new!` gets an iterator of known failures, `er_all!` gets Results to check. A `Vec<Unit>` is especially cheap here because a unit struct takes up zero bytes.

```text
SNAFU Vec       0.6 ns     0 allocations
thiserror Vec   0.6 ns     0 allocations
Er, er_new!     283 ns     1 allocation
Er, er_all!     397 ns     7 allocations
Exn            1055 ns     108 allocations
rootcause      3936 ns     304 allocations
error-stack    4230 ns     411 allocations
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

Er 0.9.0, thiserror 2.0.21, SNAFU 0.9.2, Anyhow 1.0.104, Exn 0.3.1, Eros 0.8.0, rootcause 0.13.0, error-stack 0.8.0. Measured on Linux with a Core Ultra 9 285H and rustc 1.100.0.
