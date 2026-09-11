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
        lines::for_each_line(display, |line| lines.push(line.to_owned()))?;

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
    pub struct Stop;
    let stop = Stop;
    let tree = IgnoresWriter.er();
    let mut calls = 0;
    let result = tree.er_top().try_for_each_line(|_| {
        calls += 1;
        Err(&stop)
    });

    assert!(matches!(
        result,
        Err(LineError::Callback(e)) if std::ptr::eq(e, &stop)
    ));
    assert_eq!(calls, 1);

    let mut lines = Vec::new();
    let result = tree.into_er_report().try_for_each_line(|line| {
        lines.push(line.to_owned());

        if lines.len() == 2 {
            return Err(17);
        }

        Ok(())
    });

    assert_eq!(result, Err(LineError::Callback(17)));
    assert_eq!(lines, ["first", "second"]);

    let result = lines::try_for_each_line(&"unterminated", |line| {
        assert_eq!(line, "unterminated");
        Err(&stop)
    });

    assert!(matches!(
        result,
        Err(LineError::Callback(e)) if std::ptr::eq(e, &stop)
    ));
}

#[test]
pub fn chunks() -> fmt::Result {
    let report = WriteChunks {
        parts: &["first\nsec", "ond\n", "third"],
        calls: Cell::new(0),
    }
    .er()
    .into_er_report();
    let expected = report.to_string();

    report.tree.top.calls.set(0);
    let mut lines = Vec::new();
    report.for_each_line(|line| lines.push(line.to_owned()))?;

    assert_eq!(lines, expected.lines().collect::<Vec<_>>());
    assert_eq!(report.tree.top.calls.get(), 1);

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
    let outcome = Broken
        .er()
        .into_er_top()
        .for_each_line(|line| lines.push(line.to_owned()));

    assert_eq!(outcome, Err(fmt::Error));
    assert_eq!(lines, ["done"]);

    let outcome = Broken
        .er()
        .into_er_top()
        .try_for_each_line(|_| Ok::<(), ()>(()));

    assert_eq!(outcome, Err(LineError::Format(fmt::Error)));
}
