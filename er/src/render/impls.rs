use super::{LineWriter, ReportEntry, Wrap};
use crate::{ErEntry, ErSnapshotEntry};
use core::fmt;

impl<'a, 'p, W: fmt::Write + ?Sized> LineWriter<'a, 'p, W> {
    pub const fn indent(inner: &'a mut W, prefix: &'p str) -> Self {
        Self {
            inner,
            wrap: Wrap::Indent(prefix),
            pending_breaks: 0,
            pending_cr: false,
        }
    }

    pub const fn flatten(inner: &'a mut W) -> Self {
        Self {
            inner,
            wrap: Wrap::Flatten,
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

        match self.wrap {
            Wrap::Indent(prefix) => {
                for _ in 0..self.pending_breaks {
                    self.inner.write_char('\n')?;
                    self.inner.write_str(prefix)?;
                }
            }
            Wrap::Flatten => self.inner.write_char(' ')?,
        }

        self.pending_breaks = 0;
        Ok(())
    }
}
impl<W: fmt::Write + ?Sized> fmt::Write for LineWriter<'_, '_, W> {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        let mut rest = text;

        while let Some(index) = rest.find(['\r', '\n', '\0']) {
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
            '\0' if matches!(self.wrap, Wrap::Flatten) => self.pending_breaks += 1,
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
            .map(|location| (location.file.as_str(), location.line, location.column));

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
