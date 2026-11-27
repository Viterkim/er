# Performance

This is BASICALLY NOT RELEVANT! This is only if you really really really care, and are really interested. In most cases you will never ever hit this (also for other error handling libraries).

These comparisons are also not equal in many ways, most of the crates here don't make a tree/allow many of the functionality er/exn/rootcause/error-stack does, it's still fun to look at.

## Adding context

Every call fails and the error has a u32 in it. These times include dropping the error.

### Just the error

```text
SNAFU           0.6 ns     0 allocations
thiserror       0.6 ns     0 allocations
Er              1.8 ns     0 allocations
Anyhow           13 ns     1 allocation
Eros             16 ns     1 allocation
Exn              18 ns     2 allocations
rootcause        25 ns     3 allocations
error-stack      34 ns     5 allocations
```

### One error around it

```text
SNAFU           0.8 ns     0 allocations
thiserror       0.8 ns     0 allocations
Eros             21 ns     3 allocations
Anyhow           23 ns     2 allocations
Er               30 ns     2 allocations
Exn              33 ns     5 allocations
error-stack      42 ns     9 allocations
rootcause        49 ns     7 allocations
```

thiserror and SNAFU just put the old error inside the new struct. Er builds a tree you can search later, that's the box and the vec in its allocations.

## Just creating Er errors

Keeping 128 failed results alive at a time, with dropping outside the timer:

```text
One u32 context            26 ns
Eight u32 contexts        157 ns
Eight empty contexts       71 ns
```

`.er(code)`, `.er(|_| code)` and `.er(|| MyErr::new(code))` took about the same time for one context.

## Eight layers

Eight functions add their own context above the original error and half the calls fail, time per item including the oks, allocations per failure.

### Empty structs

```text
SNAFU           0.5 ns     0 allocations
thiserror       0.5 ns     0 allocations
Eros             42 ns     3 allocations
Er               55 ns     8 allocations
Anyhow           60 ns     9 allocations
Exn              77 ns    17 allocations
error-stack      81 ns    28 allocations
rootcause       171 ns    35 allocations
```

Er allocates one list of sub errors per layer. The empty errors themselves take up no space in their boxes.

### A u32 field

```text
thiserror       1.4 ns     0 allocations
SNAFU           1.5 ns     0 allocations
Eros             51 ns    11 allocations
Anyhow           62 ns     9 allocations
Exn              93 ns    26 allocations
Er               93 ns    16 allocations
error-stack     136 ns    37 allocations
rootcause       171 ns    35 allocations
```

### Static text

```text
SNAFU           2.4 ns     0 allocations
thiserror       2.4 ns     0 allocations
Eros             41 ns     3 allocations
Anyhow           64 ns     9 allocations
Er               92 ns    16 allocations
Exn              93 ns    26 allocations
error-stack     133 ns    37 allocations
rootcause       174 ns    35 allocations
```

### Formatted Strings

```text
SNAFU            90 ns     9 allocations
Eros             90 ns    12 allocations
thiserror        93 ns     9 allocations
Anyhow          115 ns    18 allocations
Exn             145 ns    35 allocations
Er              148 ns    25 allocations
error-stack     203 ns    46 allocations
rootcause       239 ns    44 allocations
```

## Hundred empty errors

One `TopErr(pub u32)` with a hundred unit errors underneath. `er_new!` gets an iterator of known failures, `er_all!` gets Results to check. A `Vec<Unit>` is especially cheap here because a unit struct takes up zero bytes.

```text
SNAFU Vec        0.8 ns     0 allocations
thiserror Vec    0.8 ns     0 allocations
Er, er_new!      207 ns     1 allocation
Er, er_all!      415 ns     7 allocations
Exn             1073 ns   108 allocations
rootcause       4052 ns   304 allocations
error-stack     4189 ns   411 allocations
```

## Compile time

Because you want to make many many error types with Er you think "wow compile times must be TERRIBLE". With a mix of unit structs, 2 field structs and 2 variant enums with one function per error:

```text
Cold build (deps + 10 errors):
thiserror    1600 ms
Er           2265 ms
SNAFU        2316 ms

10 errors (check / build):
thiserror      29 ms /   35 ms
Er             30 ms /   37 ms
SNAFU          39 ms /   48 ms

100 errors (check / build):
thiserror      69 ms /   88 ms
Er             77 ms /  102 ms
SNAFU         171 ms /  215 ms

500 errors (check / build):
thiserror     253 ms /  331 ms
Er            311 ms /  424 ms
SNAFU         883 ms / 1096 ms
```

The cold build includes the dependencies, the other rows only rebuild the crate with the errors. Er pays once to build itself, after that an error costs about what a thiserror one does, because a plain message becomes one `f.write_str("disk full")` and errors without a format share one `Debug` function inside Er.

## Unsafe

### No unsafe

Er, thiserror, SNAFU and Exn don't use unsafe.

### Uses unsafe

Anyhow uses unsafe to keep its erased error as one pointer (custom vtable).

Eros uses it to move real errors back out of its erased union when narrowing it or making an enum.

Rootcause uses a lot for its erased Arc report, attachments and switching between typed and dynamic views.

Error-stack only has a few small casts and pinned Future helpers.

## Versions

Er 0.10.0, thiserror 2.0.21, SNAFU 0.9.2, Anyhow 1.0.105, Exn 0.3.1, Eros 0.8.0, rootcause 0.13.0, error-stack 0.8.0.
