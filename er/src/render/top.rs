use super::LineWriter;
use crate::Layout;
use core::fmt;
use fmt::Write as _;

pub fn write_top<W: fmt::Write + ?Sized>(
    writer: &mut W,
    error: &dyn fmt::Display,
    layout: Layout,
) -> fmt::Result {
    match layout {
        Layout::Multiline => write!(writer, "{error}"),
        Layout::SingleLine => {
            let mut line = LineWriter::flatten(writer);
            write!(line, "{error}")
        }
    }
}
