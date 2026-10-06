use alloc::string::String;
use core::fmt;

pub mod report;
pub mod top;

pub use report::{write_head, write_report};
pub use top::write_top;

/// Format as text, replacing a failed formatter's output with ER_FMT_FAILED.
pub fn format_string(value: &(impl fmt::Display + ?Sized)) -> String {
    let mut text = String::new();

    if fmt::write(&mut text, format_args!("{value}")).is_err() {
        text.clear();
        text.push_str("ER_FMT_FAILED");
    }

    text
}

#[derive(Clone, Copy)]
pub enum Wrap<'a> {
    Indent(&'a str),
    Flatten,
}

pub struct LineWriter<'a, 'p, W: fmt::Write + ?Sized> {
    pub inner: &'a mut W,
    pub wrap: Wrap<'p>,
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
