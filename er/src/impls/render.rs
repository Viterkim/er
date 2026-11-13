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

    pub fn flush_break(&mut self) -> fmt::Result {
        if self.pending_breaks == 0 {
            return Ok(());
        }

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
}
impl<W: fmt::Write + ?Sized> fmt::Write for LineWriter<'_, '_, W> {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        let mut rest = text;

        // Most long messages use LF. Search those in bulk once we've ruled out CR.
        let check_cr = if text.len() < 64 {
            true
        } else if self.prefix.is_empty() {
            return self.write_unindented(text);
        } else {
            text.as_bytes().contains(&b'\r')
        };
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
                self.pending_cr = false;
                self.flush_break()?;
                self.inner.write_str(&rest[..index])?;
            }

            self.write_char(char::from(rest.as_bytes()[index]))?;
            rest = &rest[index + 1..];
        }

        if !rest.is_empty() {
            self.pending_cr = false;
            self.flush_break()?;
            self.inner.write_str(rest)?;
        }

        Ok(())
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
        #[cfg(feature = "src_locations")]
        let src_location = entry
            .src_location
            .map(|location| (location.file(), location.line(), location.column()));

        Self {
            message: entry.error,
            index: entry.index,
            depth: entry.depth,
            is_last: entry.is_last,
            source_truncated: entry.source_truncated,
            #[cfg(feature = "src_locations")]
            src_location,
        }
    }
}
impl<'a> From<&'a ErSnapshotEntry> for ReportEntry<'a> {
    fn from(entry: &'a ErSnapshotEntry) -> Self {
        #[cfg(feature = "src_locations")]
        let src_location = entry
            .src_location
            .as_ref()
            .map(|location| (location.file.as_ref(), location.line, location.column));

        Self {
            message: &entry.message,
            index: entry.index,
            depth: entry.depth,
            is_last: entry.is_last,
            source_truncated: entry.source_truncated,
            #[cfg(feature = "src_locations")]
            src_location,
        }
    }
}
