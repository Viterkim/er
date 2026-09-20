# systemd and the extra spaces

You log a nice report as one journal entry, then normal `journalctl` puts the thing halfway across the screen.

```text
ConfigErr { machine: "ComputerKatten" }
`- ReadErr { path: "/etc/haandbold.toml" }
   `- No such file or directory
```

Turns into this, timestamp shortened:

```text
Sep ... Haandbold[85]: ConfigErr { machine: "ComputerKatten" }
                       `- ReadErr { path: "/etc/haandbold.toml" }
                          `- No such file or directory
```

The report is stored correctly. `journalctl` adds spaces matching the timestamp and service prefix to every line after the first. Longer prefix gives extra spaces. It's in [journalctl's renderer](https://github.com/systemd/systemd/blob/v261.2/src/shared/logs-show.c#L193-L289), you can't format it away.

## A newline before the report

If you like the report as its own block, put a newline before it:

```rust
error!("\n{}", error.er_report());
```

With a logger sending one native journal message, that looks like this:

```text
Sep ... Haandbold[85]:
                       ConfigErr { machine: "ComputerKatten" }
                       `- ReadErr { path: "/etc/haandbold.toml" }
                          `- No such file or directory
```

The prefix gets its own line, the whole tree starts below it. Still the same giant indent, still one event. I'd use this if the prefix crowds the root, but it doesn't save any horizontal space. `error!("{}", error.er_report())` is fine too.

These examples show the full message. Views that shorten multiline messages may count the empty first line toward their limit too.

## Why systemd-cat looks fine

```sh
printf 'ConfigErr\n`- ReadErr\n' | systemd-cat --identifier Haandbold
```

That's a stream. Journald [splits streams at newlines](https://github.com/systemd/systemd/blob/main/man/systemd-journald.service.xml), so those become separate entries.

One `write()` can contain both lines and still be split afterward. A native logger can instead send one entry with newlines inside its MESSAGE. The [native protocol recommends that](https://systemd.io/JOURNAL_NATIVE_PROTOCOL/), one event with one set of metadata. Sadly normal journalctl then does the giant indent.

This depends on your logger. One `error!` call means one logging event, but a backend writing it to stdout/stderr still uses the stream path. `eprintln!` uses that stream path too.

## Why not -o cat?

```sh
journalctl -u Haandbold -o cat
```

The report looks right, but timestamps and service names are gone. Annoying when an upload mixes several services. And everyone has to remember the flag.

## The journalctl fix

The actual patch is tiny:

```diff
- int len, indent = (line > 0) * prefix;
+ int len, indent = 0;
+
+ if (line > 0)
+         indent = 2;
```

That gives continuation lines two spaces instead of the whole prefix width. But now you're carrying a systemd patch, Er isn't gonna do that.

[Issue i made](https://github.com/systemd/systemd/issues/43718) asks upstream for smaller indents or an option for it. Maybe we get lucky. Until then, keeping the report together and living with the spaces is the least annoying choice here.

## The options all kinda suck

`.single_line()` avoids the indentation, but a big tree becomes one giant sausage:

```text
ConfigErr { machine: "ComputerKatten" } [ReadErr { path: "/etc/haandbold.toml" } [No such file or directory]]
```

`for_each_line()` can send every rendered line separately. That looks better in normal `journalctl`, but now one error is several events and they can interleave(you might get random bs between).

Log the normal report and try to not give up with the spaces for now. Use `-o cat`, `.single_line()`, or separate lines when one of those tradeoffs are ok (i don't like it).
