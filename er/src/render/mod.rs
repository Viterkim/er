use core::fmt;

pub mod report;
pub mod top;

pub use report::{write_head, write_report};
pub use top::write_top;

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
