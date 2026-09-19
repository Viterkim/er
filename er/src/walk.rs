use crate::types::ErNode;
#[cfg(feature = "src_locations")]
use crate::types::SrcLocation;
use alloc::vec::Vec;
use core::error::Error;
use core::marker::PhantomData;

/// Native source hops per root or stored node. Doesn't limit Er tree depth.
pub const MAX_SOURCE_HOPS: usize = 256;

/// Kind of entry in the tree
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ErEntryKind {
    Root,
    Node,
    Source,
}

/// One error, possibly with a multiline message. IDs belong to this traversal.
#[derive(Clone, Copy, Debug)]
pub struct ErEntry<'a> {
    pub error: &'a (dyn Error + 'static),
    pub kind: ErEntryKind,
    pub index: usize,
    pub parent: Option<usize>,
    pub depth: usize,
    pub is_last: bool,
    /// Another native source exists, but the walk stopped at its limit.
    pub source_truncated: bool,
    #[cfg(feature = "src_locations")]
    pub src_location: Option<SrcLocation>,
}

#[derive(Clone, Copy)]
pub struct Pending<'a> {
    pub entry: ErEntry<'a>,
    pub nodes: &'a [ErNode],
    pub sources_left: usize,
}

/// Root, native sources, then nodes.
/// Longer native chains stop at [`MAX_SOURCE_HOPS`], setting the last entry's `source_truncated`.
/// Filtering keeps the original indices, parents, depths, and `is_last` values.
#[derive(Clone)]
#[must_use]
pub struct ErEntries<'a> {
    pub pending: Vec<Pending<'a>>,
    pub next_index: usize,
}

/// Child nodes, in tree order. Excludes the root and native sources.
#[derive(Clone)]
#[must_use]
pub struct ErNodes<'a> {
    pub pending: Vec<&'a ErNode>,
}

/// Up to [`MAX_SOURCE_HOPS`] errors in a native `Error::source()` chain.
#[derive(Clone)]
#[must_use]
pub struct ErSources<'a> {
    pub next: Option<&'a (dyn Error + 'static)>,
    pub remaining: usize,
    /// Set when iteration stops at the limit with another source left.
    pub truncated: bool,
}

/// Every matching stored or native source error, in tree order.
#[must_use]
pub struct ErFindAll<'a, T> {
    pub error: Option<&'a (dyn Error + 'static)>,
    pub yielded_error: bool,
    pub sources_left: usize,
    pub nodes: &'a [ErNode],
    pub pending: Vec<&'a [ErNode]>,
    pub marker: PhantomData<fn() -> T>,
}
