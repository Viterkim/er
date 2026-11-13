pub mod report;

use alloc::string::String;
use core::fmt;
pub use report::write_report;

/// Where report strings start, most fit without regrowing the String on the way there.
pub const REPORT_CAPACITY: usize = 256;

/// Format with room for a report and replace failed output with ER_FMT_FAILED.
#[inline(never)]
pub fn format_report_string(value: &(impl fmt::Display + ?Sized)) -> String {
    write_args(
        String::with_capacity(REPORT_CAPACITY),
        format_args!("{value}"),
    )
}

pub fn format_into(text: String, value: &(impl fmt::Display + ?Sized)) -> String {
    write_args(text, format_args!("{value}"))
}

#[inline(never)]
fn write_args(mut text: String, args: fmt::Arguments<'_>) -> String {
    if fmt::write(&mut text, args).is_err() {
        text.clear();
        text.push_str("ER_FMT_FAILED");
    }

    text
}

pub struct LineWriter<'a, 'p, W: fmt::Write + ?Sized> {
    pub inner: &'a mut W,
    /// Written after every line break.
    pub prefix: &'p str,
    pub pending_breaks: usize,
    pub pending_cr: bool,
}

/// The bits needed to draw a report, whether live or saved.
pub struct ReportEntry<'a> {
    pub message: &'a dyn fmt::Display,
    pub index: usize,
    pub depth: usize,
    pub is_last: bool,
    pub source_truncated: bool,
    #[cfg(feature = "src_locations")]
    pub src_location: Option<(&'a str, u32, u32)>,
}
