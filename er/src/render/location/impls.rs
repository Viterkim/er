use super::{EntryLocation, Locations, RenderedPath, path};
use crate::render::{LineWriter, ReportEntry};
use core::fmt;

impl PartialEq for EntryLocation<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.line == other.line
            && self.column == other.column
            && (core::ptr::eq(self.file, other.file) || self.file == other.file)
    }
}

impl<'a> Locations<'a> {
    #[inline]
    pub fn hide_repeated<M: ?Sized>(&mut self, entry: &mut ReportEntry<'a, M>) {
        let location = entry
            .src_location
            .map(|(file, line, column)| EntryLocation { file, line, column });

        // The root stays outside the stack so the first level needs no allocation here.
        if let Some(depth) = entry.depth.checked_sub(1) {
            if self.ancestors.len() < depth {
                self.ancestors.push(self.previous);
            } else {
                self.ancestors.truncate(depth);
            }

            if location.is_some() && self.ancestors.last().copied().unwrap_or(self.root) == location
            {
                entry.src_location = None;
            }
        } else {
            self.root = location;
        }
        self.previous = location;
    }

    pub fn write<W: fmt::Write + ?Sized>(
        &mut self,
        writer: &mut LineWriter<'_, '_, W>,
        file: &'a str,
        line: u32,
        column: u32,
    ) -> fmt::Result {
        writer.forget_pending_break();
        writer.inner.write_str(" @ ")?;
        self.write_path(writer, file)?;

        // Space for :4294967295:4294967295.
        let mut bytes = [b':'; 22];
        let mut start = bytes.len();
        for mut value in [column, line] {
            loop {
                start -= 1;
                bytes[start] = b'0' + (value % 10) as u8;
                value /= 10;
                if value == 0 {
                    break;
                }
            }
            start -= 1;
        }

        let suffix = core::str::from_utf8(&bytes[start..]).map_err(|_| fmt::Error)?;
        writer.write_fragment(suffix)
    }

    #[inline]
    fn write_path(
        &mut self,
        writer: &mut (impl fmt::Write + ?Sized),
        file: &'a str,
    ) -> fmt::Result {
        #[cfg(not(feature = "small_path_src"))]
        if file.len() < path::MIN_CACHE_PATH {
            return writer.write_str(file);
        }

        let rendered = match self.last_path {
            Some((prev, cached)) if core::ptr::eq(prev, file) => cached,
            _ => {
                let computed = path::render_path(file);
                self.last_path = Some((file, computed));
                computed
            }
        };

        match rendered {
            RenderedPath::Plain(path) => writer.write_str(path),
            RenderedPath::Git { repo, tail } => {
                writer.write_str(repo)?;
                writer.write_str(tail)
            }
        }
    }
}
