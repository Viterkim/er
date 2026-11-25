# Performance

This is BASICALLY NOT RELEVANT! This is only if you really really really care, and are really interested. In most cases you will never ever hit this (also for other error handling libraries).

These comparisons are also not equal in many ways, most of the crates here don't make a tree/allow many of the functionality er/exn/rootcause/error-stack does, it's still fun to look at.

## Hot loops

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
thiserror       0.6 ns     0 allocations
SNAFU           0.6 ns     0 allocations
Er              1.8 ns     0 allocations
Anyhow           13 ns     1 allocation
Eros             16 ns     1 allocation
Exn              21 ns     2 allocations
rootcause        26 ns     3 allocations
error-stack      40 ns     5 allocations
```

### One error around it

```text
SNAFU           0.8 ns     0 allocations
thiserror       0.8 ns     0 allocations
Eros             22 ns     3 allocations
Er               23 ns     2 allocations
Anyhow           24 ns     2 allocations
Exn              35 ns     5 allocations
error-stack      43 ns     9 allocations
rootcause        50 ns     7 allocations
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

Eight functions add their own context above the original error and half the calls fail, time per item including the oks, allocations per failure.

### Empty structs

```text
thiserror       0.6 ns     0 allocations
SNAFU           0.6 ns     0 allocations
Eros             41 ns     3 allocations
Er               62 ns     8 allocations
Anyhow           65 ns     9 allocations
Exn              79 ns     17 allocations
error-stack      81 ns     28 allocations
rootcause       185 ns     35 allocations
```

Er allocates one list of sub errors per layer. The empty errors themselves take up no space in their boxes.

### A u32 field

```text
thiserror       1.4 ns     0 allocations
SNAFU           1.4 ns     0 allocations
Eros             50 ns     11 allocations
Anyhow           62 ns     9 allocations
Er               71 ns     16 allocations
Exn              94 ns     26 allocations
error-stack     137 ns     37 allocations
rootcause       189 ns     35 allocations
```

### Static text

```text
thiserror       2.6 ns     0 allocations
SNAFU           2.6 ns     0 allocations
Eros             39 ns     3 allocations
Anyhow           64 ns     9 allocations
Er               68 ns     16 allocations
Exn              94 ns     26 allocations
error-stack     134 ns     37 allocations
rootcause       177 ns     35 allocations
```

### Formatted Strings

```text
Eros             91 ns     12 allocations
SNAFU            94 ns     9 allocations
thiserror       100 ns     9 allocations
Anyhow          120 ns     18 allocations
Exn             153 ns     35 allocations
Er              174 ns     25 allocations
error-stack     212 ns     46 allocations
rootcause       248 ns     44 allocations
```

## Hundred empty errors

One `TopErr(pub u32)` with a hundred unit errors underneath. `er_new!` gets an iterator of known failures, `er_all!` gets Results to check. A `Vec<Unit>` is especially cheap here because a unit struct takes up zero bytes.

```text
SNAFU Vec       0.8 ns     0 allocations
thiserror Vec   0.8 ns     0 allocations
Er, er_new!     220 ns     1 allocation
Er, er_all!     398 ns     7 allocations
Exn            1078 ns     108 allocations
rootcause      4111 ns     304 allocations
error-stack    4336 ns     411 allocations
```

## Compile time

Because you want to make many many error types with Er you think "wow compile times must be TERRIBLE". With a mix of unit structs, 2 field structs and 2 variant enums with one function per error:

```text
Cold build (deps + 10 errors):
thiserror    1962 ms
Er           2072 ms
SNAFU        3094 ms

10 errors (check / build):
Er             40 ms /   62 ms
thiserror      45 ms /   58 ms
SNAFU          63 ms /   74 ms

100 errors (check / build):
Er             98 ms /  146 ms
thiserror     131 ms /  156 ms
SNAFU         269 ms /  301 ms

500 errors (check / build):
Er            265 ms /  519 ms
thiserror     511 ms /  541 ms
SNAFU        1172 ms / 1144 ms
```

This is because Er writes `Debug` and `Display` directly in one derive instead of expanding `format_args!` + a separate `#[derive(Debug)]` on every error like `thiserror` (or generating selector structs like `SNAFU`).

## Unsafe

### No unsafe

Er, thiserror, SNAFU and Exn don't use unsafe.

### Uses unsafe

Anyhow uses unsafe to keep its erased error as one pointer (custom vtable).

Eros uses it to move real errors back out of its erased union when narrowing it or making an enum.

Rootcause uses a lot for its erased Arc report, attachments and switching between typed and dynamic views.

Error-stack only has a few small casts and pinned Future helpers.

## Versions

Er 0.9.0, thiserror 2.0.21, SNAFU 0.9.2, Anyhow 1.0.105, Exn 0.3.1, Eros 0.8.0, rootcause 0.13.0, error-stack 0.8.0.
