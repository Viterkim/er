# Future ideas

## More convenient enum variant creation

Maybe generate a `config_err` module for `ConfigErr`

```rust
use config_err::*;

port.parse::<u16>().er(|_| invalid_port(port))?;
mode.parse::<bool>().er(|_| invalid_mode(mode))?;
```

but who would use that so whatever. and its more generated stuff.

And doing an alias to E with E::invalid_port() is the same, so eh.

## Fix wrap / std_error, even though its a really rare edge case

With `std_error`, `.er()` and `.er_add(wrapped)` box the Wrap. With `output = report` the log looks fine, then `.er_find()` misses the error you just saw. Use `.er_wrap(..)` on the Result or `.er_add(wrapped.tree)`.

The `Unpin` trick opened the Wrap, but then even `.er_add(SendError(wrapped))` stopped compiling. Checking `source()` on the way in called it on every ordinary error. Checking while walking could open a `Box<dyn Error>`, but walking 100 ordinary errors with sources took 39% longer, and adding a sibling hid the child again. Leaving this one alone for now.

Patches, repros and timings are in the local `er-extras` checkout under `results/std-error-wrap/`.
