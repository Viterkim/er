pub use super::ReportEntry;

use super::location::Locations;
use super::{Gutter, LineWriter};
use crate::impls::saved::for_distinct_parts;
use crate::{ErEntries, ErEntryKind, ErNode, ErSaved, ErSrcLocation};
use alloc::vec::Vec;
use core::{error::Error, fmt};
use fmt::Write as _;

pub fn write_entry<W: fmt::Write + ?Sized>(
    entry: ReportEntry<'_>,
    writer: &mut LineWriter<'_, '_, W>,
) -> fmt::Result {
    write_entry_cached(entry, writer, &mut Locations::default())
}

fn write_entry_cached<'a, W: fmt::Write + ?Sized, M: Message + ?Sized>(
    entry: ReportEntry<'a, M>,
    writer: &mut LineWriter<'_, '_, W>,
    locations: &mut Locations<'a>,
) -> fmt::Result {
    entry.message.write_message(writer)?;

    if let Some((file, line, column)) = entry.src_location {
        locations.write(writer, file, line, column)?;
    }

    if entry.source_truncated {
        writer.forget_pending_break();
        writer.inner.write_str(" [source limit reached]")?;
    }

    Ok(())
}

pub(crate) trait Message {
    fn write_message<W: fmt::Write + ?Sized>(&self, writer: &mut W) -> fmt::Result;
}
impl Message for str {
    fn write_message<W: fmt::Write + ?Sized>(&self, writer: &mut W) -> fmt::Result {
        writer.write_str(self)
    }
}
impl Message for dyn fmt::Display + '_ {
    fn write_message<W: fmt::Write + ?Sized>(&self, writer: &mut W) -> fmt::Result {
        let mut message = CheckedWriter {
            writer: &mut *writer,
            failed: false,
        };
        let formatted = write!(message, "{self}");
        if message.failed {
            return Err(fmt::Error);
        }

        if formatted.is_err() {
            // A broken Display can leave half a message. Keep printing the rest of the tree.
            writer.write_str("ER_FMT_FAILED")?;
        }

        Ok(())
    }
}

// Display can swallow a write error and return Ok.
struct CheckedWriter<'a, W: ?Sized> {
    writer: &'a mut W,
    failed: bool,
}
impl<W: fmt::Write + ?Sized> fmt::Write for CheckedWriter<'_, W> {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        if self.failed {
            return Err(fmt::Error);
        }

        let result = self.writer.write_str(text);
        self.failed = result.is_err();

        result
    }
}

#[inline(never)]
pub fn write_report<W: fmt::Write + ?Sized>(
    writer: &mut W,
    error: &(dyn Error + 'static),
    nodes: &[ErNode],
    saved: &[ErSaved],
    extra: &[ErSaved],
    src_location: Option<ErSrcLocation>,
) -> fmt::Result {
    let mut entries = ErEntries::new(error, nodes, src_location);
    if nodes.is_empty() && saved.is_empty() && extra.is_empty() {
        if let Some(root) = entries.next() {
            write_entry(root.into(), &mut LineWriter::indent(writer, ""))?;
        }

        if entries.next_error.is_none() {
            return Ok(());
        }
    }

    write_entries_with_saved(writer, entries, |entry| {
        let (first, second) = if entry.kind == ErEntryKind::Root {
            (saved, extra)
        } else if entry.saved.is_none() && entry.shared_saved.is_none() {
            return (entry.into(), None);
        } else {
            entry.saved_slices()
        };
        let attachments = !first.is_empty() || !second.is_empty();

        (entry.into(), attachments.then_some((first, second)))
    })
}

pub fn valid_depth(previous: Option<usize>, depth: usize) -> bool {
    match previous {
        None => depth == 0,
        Some(previous) => depth > 0 && depth <= previous.saturating_add(1),
    }
}

pub fn write_entries<'a, W: fmt::Write + ?Sized>(
    writer: &mut W,
    entries: impl IntoIterator<Item = impl Into<ReportEntry<'a>>>,
) -> fmt::Result {
    let mut rows = ReportRows::new(writer);
    let mut previous_depth = None;
    for entry in entries {
        let entry = entry.into();
        if !valid_depth(previous_depth, entry.depth) {
            return Err(fmt::Error);
        }
        previous_depth = Some(entry.depth);

        rows.entry(entry)?;
    }

    Ok(())
}

pub(crate) fn write_entries_with_saved<'a, W: fmt::Write + ?Sized, T, M: Message + ?Sized + 'a>(
    writer: &mut W,
    entries: impl IntoIterator<Item = T>,
    into_entry: impl FnMut(T) -> (ReportEntry<'a, M>, Option<(&'a [ErSaved], &'a [ErSaved])>),
) -> fmt::Result {
    let mut rows = ReportRows::new(writer);
    let mut entries = entries.into_iter().map(into_entry);
    let first = loop {
        let Some((entry, attachments)) = entries.next() else {
            return Ok(());
        };
        let depth = entry.depth;
        rows.entry(entry)?;

        if let Some(attachments) = attachments {
            break (depth, attachments);
        }
    };

    let mut current = Some(first);
    let mut parents = Vec::new();
    for (mut entry, attachments) in entries {
        while let Some((depth, attachments)) = current {
            if depth < entry.depth {
                if depth + 1 == entry.depth {
                    entry.is_last = false;
                }
                break;
            }

            rows.attachments(depth + 1, attachments)?;
            current = parents.pop();
        }

        let depth = entry.depth;
        rows.entry(entry)?;
        if let Some(attachments) = attachments
            && let Some(parent) = current.replace((depth, attachments))
        {
            parents.push(parent);
        }
    }

    while let Some((depth, attachments)) = current {
        rows.attachments(depth + 1, attachments)?;
        current = parents.pop();
    }

    Ok(())
}

struct ReportRows<'a, W: ?Sized> {
    writer: &'a mut W,
    gutter: Gutter,
    locations: Locations<'a>,
}
impl<'a, W: fmt::Write + ?Sized> ReportRows<'a, W> {
    fn new(writer: &'a mut W) -> Self {
        Self {
            writer,
            gutter: Gutter::default(),
            locations: Locations::default(),
        }
    }

    #[inline]
    fn entry<M: Message + ?Sized>(&mut self, mut entry: ReportEntry<'a, M>) -> fmt::Result {
        self.locations.hide_repeated(&mut entry);

        let gutter = self
            .gutter
            .branch(self.writer, entry.depth, entry.is_last)?;
        let mut line = LineWriter::indent(self.writer, &gutter[..entry.depth * 3]);
        write_entry_cached(entry, &mut line, &mut self.locations)
    }

    fn attachment(&mut self, depth: usize, saved: &ErSaved, is_last: bool) -> fmt::Result {
        let gutter = self.gutter.branch(self.writer, depth, is_last)?;
        let mut line = LineWriter::indent(self.writer, &gutter[..depth * 3]);
        line.write_str(&saved.title)?;
        line.forget_pending_break();
        line.write_str(" [saved]")?;

        let mut body = LineWriter::indent(self.writer, gutter);
        body.write_char('\n')?;
        write!(body, "{}", saved.kind)
    }

    fn attachments(
        &mut self,
        depth: usize,
        (first, second): (&[ErSaved], &[ErSaved]),
    ) -> fmt::Result {
        for_distinct_parts(first, second, |saved, is_last| {
            self.attachment(depth, saved, is_last)
        })
    }
}
