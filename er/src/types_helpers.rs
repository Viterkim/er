use crate::{ErEntry, ErNode};
use alloc::string::String;
use core::fmt;

pub struct Pending<'a> {
    pub entry: ErEntry<'a>,
    pub nodes: &'a [ErNode],
    pub sources_left: usize,
}

pub struct Lines<F, E> {
    pub buffer: String,
    pub emit: F,
    pub error: Option<E>,
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
