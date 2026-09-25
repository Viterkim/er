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

Make normal .er() work with a Wrap that has std_error. Right now it boxes the Wrap, so .er_find() sees the Wrap but can't see the errors inside it. .er_wrap() is the cope solution that sucks, but having to remember a different function is not good enough. Tried letting source() point back to the tree. Other libraries printed an extra cause, so that is not good enough. Maybe Er can spot its own Wrap before boxing it, but how or is that gonna fuck up everything else.
