# Future ideas

## More convenient enum variant creation

Maybe a more convenient enum varint thing (maybe type E = VeryLongErrorName; thing.er(|| E::invalid_input(input))?;) but probably not, that idea sucks, but its the last 'ergonomic annoyance' i have.

## Fix wrap / std_err, even though its a really rare edge case

Make normal .er() work with a Wrap that has std_error. Right now it boxes the Wrap, so .er_find() sees the Wrap but can't see the errors inside it. .er_from_wrap() is the cope solution that sucks, but having to remember a different function is not good enough. Tried letting source() point back to the tree. Other libraries printed an extra cause, so that is not good enough. Maybe Er can spot its own Wrap before boxing it, but how or is that gonna fuck up everything else.
