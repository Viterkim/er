use super::LineWriter;
pub use super::ReportEntry;
use crate::impls::saved::write_saved_parts;
use crate::{ErEntries, ErNode, ErSaved, ErSrcLocation};
#[cfg(feature = "src_locations")]
use alloc::vec::Vec;
use alloc::{borrow::Cow, string::String};
use core::{error::Error, fmt};
use fmt::Write as _;

pub fn write_entry<W: fmt::Write + ?Sized>(
    entry: ReportEntry<'_>,
    writer: &mut LineWriter<'_, '_, W>,
) -> fmt::Result {
    let mut message = Tracked {
        writer: &mut *writer,
        failed: false,
    };
    if write!(message, "{}", entry.message).is_err() {
        if message.failed {
            return Err(fmt::Error);
        }

        // That error came from the message's own Display. One broken error somewhere below shouldn't cost the
        // whole report, saved reports and all. A snapshot only loses that one message, so do the same.
        writer.write_str("ER_FMT_FAILED")?;
    }

    #[cfg(feature = "src_locations")]
    if let Some((file, line, column)) = entry.src_location {
        #[cfg(feature = "small_path_src")]
        let file = small_path(file);
        writer.forget_pending_break();
        writer.inner.write_str(" @ ")?;
        writer.write_str(file)?;
        writer.write_char(':')?;
        write!(writer.inner, "{line}:{column}")?;
    }

    if entry.source_truncated {
        writer.forget_pending_break();
        writer.inner.write_str(" [source limit reached]")?;
    }

    Ok(())
}

/// Tells a failing destination apart from a failing Display, both come out as the same `fmt::Error`.
struct Tracked<'w, 'a, 'p, W: fmt::Write + ?Sized> {
    writer: &'w mut LineWriter<'a, 'p, W>,
    failed: bool,
}
impl<W: fmt::Write + ?Sized> fmt::Write for Tracked<'_, '_, '_, W> {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        let result = self.writer.write_str(text);
        self.failed |= result.is_err();

        result
    }
}

// Last src directory, plus its parent. This is display-only, not crate-root detection.
#[cfg(all(feature = "src_locations", feature = "small_path_src"))]
fn small_path(file: &str) -> &str {
    let mut offset = 0;
    let mut parent = None;
    let mut shortened = file;

    for part in file.split_inclusive(['/', '\\']) {
        let name = part.trim_end_matches(['/', '\\']);
        if name == "src" && name.len() < part.len() {
            shortened = match parent {
                Some(start) => &file[start..],
                None => &file[offset..],
            };
        }

        if name.len() == 2 && name.as_bytes()[0].is_ascii_alphabetic() && name.ends_with(':') {
            parent = None; // A Windows drive isn't a parent directory.
        } else if !name.is_empty() {
            parent = Some(offset);
        }
        offset += part.len();
    }

    shortened
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
    let entries = ErEntries::new(error, nodes, src_location);
    write_report_entries::<false, _>(writer, entries)?;
    write_saved_parts(writer, saved, extra)
}

pub fn valid_depth(previous: Option<usize>, depth: usize) -> bool {
    match previous {
        None => depth == 0,
        Some(previous) => depth > 0 && depth <= previous.saturating_add(1),
    }
}

#[cfg(feature = "src_locations")]
#[derive(Clone, Copy, PartialEq, Eq)]
struct EntryLocation<'a> {
    file: &'a str,
    line: u32,
    column: u32,
}

#[cfg(feature = "src_locations")]
fn hide_repeated_location<'a>(
    entry: &mut ReportEntry<'a>,
    ancestors: &mut Vec<Option<EntryLocation<'a>>>,
    root: &mut Option<EntryLocation<'a>>,
    previous: &mut Option<EntryLocation<'a>>,
) {
    let location = entry
        .src_location
        .map(|(file, line, column)| EntryLocation { file, line, column });

    // The root stays outside the stack so the first level needs no allocation here.
    if let Some(depth) = entry.depth.checked_sub(1) {
        if ancestors.len() < depth {
            ancestors.push(*previous);
        } else {
            ancestors.truncate(depth);
        }

        if location.is_some() && ancestors.last().copied().unwrap_or(*root) == location {
            entry.src_location = None;
        }
    } else {
        *root = location;
    }
    *previous = location;
}

pub fn write_entries<'a, W: fmt::Write + ?Sized>(
    writer: &mut W,
    entries: impl IntoIterator<Item = impl Into<ReportEntry<'a>>>,
) -> fmt::Result {
    write_report_entries::<true, _>(writer, entries)
}

pub(crate) fn write_report_entries<'a, const CHECK_DEPTH: bool, W: fmt::Write + ?Sized>(
    writer: &mut W,
    entries: impl IntoIterator<Item = impl Into<ReportEntry<'a>>>,
) -> fmt::Result {
    let mut prefix = Cow::Borrowed("");
    let mut previous_depth = None;
    #[cfg(feature = "src_locations")]
    let (mut ancestors, mut root_location, mut previous_location) = (Vec::new(), None, None);

    for entry in entries {
        let entry = entry.into();
        if CHECK_DEPTH && !valid_depth(previous_depth, entry.depth) {
            return Err(fmt::Error);
        }
        previous_depth = Some(entry.depth);
        #[cfg(feature = "src_locations")]
        let mut entry = entry;
        #[cfg(feature = "src_locations")]
        hide_repeated_location(
            &mut entry,
            &mut ancestors,
            &mut root_location,
            &mut previous_location,
        );
        if entry.depth > 0 {
            let parent_width = (entry.depth - 1) * 3;
            let (branch, continuation) = if entry.is_last {
                ("\n`- ", "   ")
            } else {
                ("\n|- ", "|  ")
            };

            if parent_width == 0 {
                writer.write_str(branch)?;
            } else {
                writer.write_char('\n')?;
                writer.write_str(&prefix[..parent_width])?;
                writer.write_str(&branch[1..])?;
            }
            match &mut prefix {
                Cow::Borrowed(previous) if parent_width == 0 => *previous = continuation,
                Cow::Borrowed(previous) => {
                    let mut nested = String::with_capacity(16);
                    nested.push_str(&previous[..parent_width]);
                    nested.push_str(continuation);
                    prefix = Cow::Owned(nested);
                }
                Cow::Owned(prefix) => {
                    prefix.truncate(parent_width);
                    prefix.push_str(continuation);
                }
            }
        }

        let mut line = LineWriter::indent(writer, &prefix);
        write_entry(entry, &mut line)?;
    }

    Ok(())
}
