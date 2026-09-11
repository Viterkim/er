use crate::{ErEntry, ErNode};
use alloc::vec::Vec;
use core::error::Error;

pub mod impls;

/// Native source hops per root or stored node. Doesn't limit Er tree depth.
pub const MAX_SOURCE_HOPS: usize = 256;

pub struct Pending<'a> {
    pub entry: ErEntry<'a>,
    pub nodes: &'a [ErNode],
    pub sources_left: usize,
}

/// Root, native sources, then nodes.
/// Longer native chains stop at [`MAX_SOURCE_HOPS`], setting the last entry's `source_truncated`.
/// Filtering keeps the original indices, parents, depths, and `is_last` values.
#[must_use]
pub struct ErEntries<'a> {
    pub pending: Vec<Pending<'a>>,
    pub next_index: usize,
}

/// Child nodes, in tree order. Excludes the root and native sources.
#[must_use]
pub struct ErNodes<'a> {
    pub pending: Vec<&'a ErNode>,
}

/// Up to [`MAX_SOURCE_HOPS`] errors in a native `Error::source()` chain.
#[must_use]
pub struct ErSources<'a> {
    pub next: Option<&'a (dyn Error + 'static)>,
    pub remaining: usize,
    /// Set when iteration stops at the limit with another source left.
    pub truncated: bool,
}
