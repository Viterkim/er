pub mod report;

mod gutter;
mod location;

pub use report::write_report;

use alloc::string::String;
use core::fmt;

#[derive(Default)]
struct Gutter {
    shallow: &'static str,
    deep: String,
}

/// Where report strings start, most fit without regrowing the String on the way there.
pub const REPORT_CAPACITY: usize = 256;

/// Format with room for a report and replace failed output with ER_FMT_FAILED.
#[inline(never)]
pub fn format_report_string(value: &(impl fmt::Display + ?Sized)) -> String {
    format_into(String::with_capacity(REPORT_CAPACITY), value)
}

pub fn format_into(mut text: String, value: &(impl fmt::Display + ?Sized)) -> String {
    write_args(&mut text, format_args!("{value}"));
    text
}

#[inline(never)]
pub(crate) fn write_args(text: &mut String, args: fmt::Arguments<'_>) {
    if fmt::write(text, args).is_err() {
        text.clear();
        text.push_str("ER_FMT_FAILED");
    }
}

pub struct LineWriter<'a, 'p, W: fmt::Write + ?Sized> {
    pub inner: &'a mut W,
    /// Written after every line break.
    pub prefix: &'p str,
    pub pending_breaks: usize,
    pub pending_cr: bool,
}

/// The bits needed to draw a report, whether live or saved.
pub struct ReportEntry<'a, M: ?Sized = dyn fmt::Display + 'a> {
    pub message: &'a M,
    pub index: usize,
    pub depth: usize,
    pub is_last: bool,
    pub source_truncated: bool,
    pub src_location: Option<(&'a str, u32, u32)>,
}
