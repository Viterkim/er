# systemd and the extra spaces

Log a report as one journal entry and regular `journalctl` can put it halfway across the screen.

```text
Sep ... Haandbold[85]: ConfigErr { machine: "ComputerKatten" }
                       `- ReadErr { path: "/etc/haandbold.toml" }
                          `- No such file or directory
```

Those spaces come from the width of the timestamp and service name. The journal still has the original report. A newline in `error!("\n{}", error.er_report())` puts the whole tree below that prefix, with the same giant indent. [journalctl renderer](https://github.com/systemd/systemd/blob/v261.2/src/shared/logs-show.c#L193-L289)

## One report or one entry per line

A logger sending a native journal message can keep the whole report in one event. Writing to stdout, stderr or `systemd-cat` takes the stream path. Journald splits it at newlines even if you got everything into one `write()`. [Native journal messages](https://systemd.io/JOURNAL_NATIVE_PROTOCOL/)

You can use `for_each_line()` to send a separate event for each line yourself. The indentation looks better, and a message from another service can land right between your errors.

```sh
journalctl -u Haandbold -o cat
```

That looks right until you need a timestamp or want to see which service said it. I keep the report in one event and live with the spaces.

## The journalctl fix

The actual patch is tiny.

```diff
- int len, indent = (line > 0) * prefix;
+ int len, indent = 0;
+
+ if (line > 0)
+         indent = 2;
```

Two spaces on the continuation lines. Now we just need a way to do that without carrying a systemd patch. [Issue i made](https://github.com/systemd/systemd/issues/43718)
