use crate::render::{LineWriter, ReportEntry};
use crate::{ErEntry, ErSnapshotEntry};
use core::fmt::{self, Write as _};

impl<'a, 'p, W: fmt::Write + ?Sized> LineWriter<'a, 'p, W> {
    pub const fn indent(inner: &'a mut W, prefix: &'p str) -> Self {
        Self {
            inner,
            prefix,
            pending_breaks: 0,
            pending_cr: false,
        }
    }

    pub const fn forget_pending_break(&mut self) {
        self.pending_breaks = 0;
        self.pending_cr = false;
    }

    #[inline]
    pub fn flush_break(&mut self) -> fmt::Result {
        if self.pending_breaks == 0 {
            return Ok(());
        }

        self.write_breaks()
    }

    #[inline]
    pub(crate) fn write_fragment(&mut self, text: &str) -> fmt::Result {
        self.pending_cr = false;
        self.flush_break()?;
        self.inner.write_str(text)
    }

    #[cold]
    fn write_breaks(&mut self) -> fmt::Result {
        if self.pending_breaks > 1 {
            let prefix = self.prefix.trim_end();
            for _ in 1..self.pending_breaks {
                self.inner.write_char('\n')?;
                if !prefix.is_empty() {
                    self.inner.write_str(prefix)?;
                }
            }
        }

        self.inner.write_char('\n')?;
        if !self.prefix.is_empty() {
            self.inner.write_str(self.prefix)?;
        }

        self.pending_breaks = 0;
        Ok(())
    }

    #[cold]
    #[inline(never)]
    fn write_unindented(&mut self, text: &str) -> fmt::Result {
        for (index, part) in text.split('\r').enumerate() {
            if index != 0 {
                self.write_char('\r')?;
            }
            if part.is_empty() {
                continue;
            }

            let part = if self.pending_cr {
                self.pending_cr = false;
                part.strip_prefix('\n').unwrap_or(part)
            } else {
                part
            };
            let content = part.trim_end_matches('\n');

            if !content.is_empty() {
                self.flush_break()?;
                self.inner.write_str(content)?;
            }
            self.pending_breaks += part.len() - content.len();
        }

        Ok(())
    }

    #[inline(never)]
    fn write_lines(&mut self, text: &str) -> fmt::Result {
        if text.len() >= 64 && self.prefix.is_empty() {
            return self.write_unindented(text);
        }

        let mut rest = text;
        let check_cr = text.len() < 24 || text.as_bytes().contains(&b'\r');
        loop {
            let next = if check_cr {
                rest.as_bytes()
                    .iter()
                    .position(|byte| matches!(byte, b'\r' | b'\n'))
            } else {
                rest.find('\n')
            };
            let Some(index) = next else { break };
            if index > 0 {
                self.write_fragment(&rest[..index])?;
            }

            let tail = &rest[index..];
            if let Some(after) = tail.strip_prefix("\r\n") {
                self.pending_cr = false;
                self.pending_breaks += 1;
                rest = after;
            } else {
                self.write_char(char::from(tail.as_bytes()[0]))?;
                rest = &tail[1..];
            }
        }

        if !rest.is_empty() {
            self.write_fragment(rest)?;
        }

        Ok(())
    }
}
impl<W: fmt::Write + ?Sized> fmt::Write for LineWriter<'_, '_, W> {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        if text.is_empty() {
            return Ok(());
        }
        if text.len() < 24 && text.bytes().all(|byte| byte >= b' ') {
            return self.write_fragment(text);
        }

        self.write_lines(text)
    }

    fn write_char(&mut self, character: char) -> fmt::Result {
        let after_carriage_return = self.pending_cr;
        self.pending_cr = false;

        match character {
            '\r' => {
                self.pending_breaks += 1;
                self.pending_cr = true;
            }
            '\n' => {
                if !after_carriage_return {
                    self.pending_breaks += 1;
                }
            }
            _ => {
                self.flush_break()?;
                self.inner.write_char(character)?;
            }
        }

        Ok(())
    }
}

impl<'a> From<ErEntry<'a>> for ReportEntry<'a> {
    fn from(entry: ErEntry<'a>) -> Self {
        let src_location = entry
            .src_location
            .map(|location| (location.file(), location.line(), location.column()));

        Self {
            message: entry.error,
            index: entry.index,
            depth: entry.depth,
            is_last: entry.is_last,
            source_truncated: entry.source_truncated,
            src_location,
        }
    }
}
impl<'a> From<&'a ErSnapshotEntry> for ReportEntry<'a> {
    fn from(entry: &'a ErSnapshotEntry) -> Self {
        entry.row_with_message(&entry.message)
    }
}
impl ErSnapshotEntry {
    pub(crate) fn report_entry(&self) -> ReportEntry<'_, str> {
        self.row_with_message(&self.message)
    }

    fn row_with_message<'a, M: ?Sized>(&'a self, message: &'a M) -> ReportEntry<'a, M> {
        let src_location = self
            .src_location
            .as_ref()
            .map(|location| (location.file.as_ref(), location.line, location.column));

        ReportEntry {
            message,
            index: self.index,
            depth: self.depth,
            is_last: self.is_last,
            source_truncated: self.source_truncated,
            src_location,
        }
    }
}
