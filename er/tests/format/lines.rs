use er::*;
use std::{cell::Cell, error::Error, fmt};

#[derive(Debug)]
pub struct WriteChunks {
    pub parts: &'static [&'static str],
    pub calls: Cell<usize>,
}
impl fmt::Display for WriteChunks {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.calls.set(self.calls.get() + 1);

        for part in self.parts {
            f.write_str(part)?;
        }

        Ok(())
    }
}
impl Error for WriteChunks {}

#[test]
pub fn lines() -> fmt::Result {
    for parts in [
        &[""][..],
        &["\n"],
        &["\n\n"],
        &["hello"],
        &["a\n\nb\n"],
        &["æ🦀\r", "\n", "z\r"],
        &["a\0b", "\r", "c\n"],
    ] {
        let value = WriteChunks {
            parts,
            calls: Cell::new(0),
        };
        let expected = parts.concat();
        let mut lines = Vec::new();
        let display: &dyn fmt::Display = &value;
        lines::for_each_line(display, |l| lines.push(l.to_owned()))?;

        assert_eq!(lines, expected.lines().collect::<Vec<_>>());
        assert_eq!(value.calls.get(), 1);
    }

    Ok(())
}

#[derive(Debug)]
pub struct IgnoresWriter;
impl fmt::Display for IgnoresWriter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let _ = f.write_str("first\nsecond\n");
        let _ = f.write_str("third\npartial");

        Ok(())
    }
}
impl Error for IgnoresWriter {}

#[test]
pub fn callback_failure() {
    let tree = ErTree::from(IgnoresWriter);
    let mut calls = 0;
    let result = tree.try_for_each_line(|_| {
        calls += 1;
        Err(1)
    });

    assert_eq!(result, Err(ErLineError::Callback(1)));
    assert_eq!(calls, 1);

    let mut lines = Vec::new();
    let result = tree.try_for_each_line(|l| {
        lines.push(l.to_owned());

        if lines.len() == 2 {
            return Err(17);
        }

        Ok(())
    });

    let failure =
        lines::try_for_each_line(&"report", |_| Err(std::io::Error::other("sink closed")))
            .unwrap_err();
    let tree = ErTree::from(failure);
    assert_eq!(tree.er_report().matches("sink closed").count(), 1);
    assert!(tree.er_contains::<std::io::Error>());

    assert_eq!(result, Err(ErLineError::Callback(17)));
    assert_eq!(lines, ["first", "second"]);

    let result = lines::try_for_each_line(&"unterminated", |l| {
        assert_eq!(l, "unterminated");
        Err(2)
    });

    assert_eq!(result, Err(ErLineError::Callback(2)));
}

#[test]
pub fn chunks() -> fmt::Result {
    let report = ErTree::from(WriteChunks {
        parts: &["first\nsec", "ond\n", "third"],
        calls: Cell::new(0),
    });
    let expected = report.er_report();

    report.top.calls.set(0);
    let mut lines = Vec::new();
    report.for_each_line(|l| lines.push(l.to_owned()))?;

    assert_eq!(lines, expected.lines().collect::<Vec<_>>());
    assert_eq!(report.top.calls.get(), 1);

    Ok(())
}

#[derive(Debug)]
pub struct Broken;
impl fmt::Display for Broken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("done\npartial")?;
        Err(fmt::Error)
    }
}
impl Error for Broken {}

#[test]
pub fn format_failure() {
    let mut lines = Vec::new();
    let outcome = ErTree::from(Broken).for_each_line(|l| lines.push(l.to_owned()));

    assert_eq!(outcome, Ok(()));
    assert!(lines[1].starts_with("partialER_FMT_FAILED"));
}
