use super::LineWriter;
pub use super::ReportEntry;
use crate::{ErEntries, ErEntry, ErNode, Layout, SrcLocation};
use alloc::string::String;
use core::{error::Error, fmt};
use fmt::Write as _;

/// Write one entry's message and location, for custom renderers.
pub fn write_head<W: fmt::Write + ?Sized>(
    entry: ErEntry<'_>,
    writer: &mut LineWriter<'_, '_, W>,
) -> fmt::Result {
    write_entry(entry.into(), writer)
}

pub fn write_entry<W: fmt::Write + ?Sized>(
    entry: ReportEntry<'_>,
    writer: &mut LineWriter<'_, '_, W>,
) -> fmt::Result {
    write!(writer, "{}", entry.message)?;

    #[cfg(feature = "src_locations")]
    if let Some((file, line, column)) = entry.src_location {
        #[cfg(feature = "small_path_src")]
        let file = small_path(file);
        writer.forget_pending_break();
        write!(writer, " @ {file}:{line}:{column}")?;
    }

    if entry.source_truncated {
        writer.forget_pending_break();
        writer.write_str(" [source limit reached]")?;
    }

    Ok(())
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

pub fn write_report<W: fmt::Write + ?Sized>(
    writer: &mut W,
    error: &(dyn Error + 'static),
    nodes: &[ErNode],
    layout: Layout,
    src_location: Option<SrcLocation>,
) -> fmt::Result {
    let entries = ErEntries::new(error, nodes, src_location);

    write_entries(writer, entries, layout)
}

pub fn write_entries<'a, W: fmt::Write + ?Sized>(
    writer: &mut W,
    entries: impl IntoIterator<Item = impl Into<ReportEntry<'a>>>,
    layout: Layout,
) -> fmt::Result {
    match layout {
        Layout::Multiline => write_multiline(writer, entries),
        Layout::SingleLine => write_single_line(writer, entries),
    }
}

pub fn write_multiline<'a, W: fmt::Write + ?Sized>(
    writer: &mut W,
    entries: impl IntoIterator<Item = impl Into<ReportEntry<'a>>>,
) -> fmt::Result {
    let mut prefix = String::new();

    for entry in entries {
        let entry = entry.into();
        if entry.depth > 0 {
            prefix.truncate((entry.depth - 1) * 3);
            writer.write_char('\n')?;
            writer.write_str(&prefix)?;

            let (branch, continuation) = if entry.is_last {
                ("`- ", "   ")
            } else {
                ("|- ", "|  ")
            };

            writer.write_str(branch)?;
            prefix.push_str(continuation);
        }

        let mut line = LineWriter::indent(writer, &prefix);
        write_entry(entry, &mut line)?;
    }

    Ok(())
}

pub fn write_single_line<'a, W: fmt::Write + ?Sized>(
    writer: &mut W,
    entries: impl IntoIterator<Item = impl Into<ReportEntry<'a>>>,
) -> fmt::Result {
    let mut depth = 0;

    for entry in entries {
        let entry = entry.into();
        if entry.depth > depth {
            writer.write_str(" [")?;
        } else if entry.index > 0 {
            for _ in entry.depth..depth {
                writer.write_char(']')?;
            }
            writer.write_str(" | ")?;
        }

        depth = entry.depth;
        let mut line = LineWriter::flatten(writer);
        write_entry(entry, &mut line)?;
    }

    for _ in 0..depth {
        writer.write_char(']')?;
    }

    Ok(())
}
