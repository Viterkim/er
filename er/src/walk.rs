use crate::types::ErNode;
#[cfg(feature = "src_locations")]
use crate::types::ErSrcLocation;
use alloc::{collections::BTreeSet, vec::Vec};
use core::error::Error;
use core::{marker::PhantomData, slice};

/// Native source hops per root or stored node. Doesn't limit Er tree depth.
pub const MAX_SOURCE_HOPS: usize = 256;

/// Kind of entry in the tree
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ErEntryKind {
    Root,
    Node,
    Source,
}

/// One error, whose message can span several lines. Indices are for this traversal and count
/// native sources.
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
    pub src_location: Option<ErSrcLocation>,
}

#[derive(Clone, Copy)]
pub struct PendingError<'a> {
    pub error: &'a (dyn Error + 'static),
    pub depth: usize,
    pub sources_left: usize,
    pub is_last: bool,
    #[cfg(feature = "src_locations")]
    pub src_location: Option<ErSrcLocation>,
}

#[derive(Clone, Copy)]
pub struct PendingNodes<'a> {
    pub siblings: &'a [ErNode],
    pub parent: usize,
    pub depth: usize,
}

/// Each error, then its native sources and stored sub errors.
/// At [`MAX_SOURCE_HOPS`], the iterator stops following that chain and sets the last entry's
/// `source_truncated` if another source remains.
/// Filtering keeps the original indices, links between entries, depths, and `is_last` values.
#[derive(Clone)]
#[must_use]
pub struct ErEntries<'a> {
    pub next_error: Option<PendingError<'a>>,
    pub nodes: PendingNodes<'a>,
    pub pending: Vec<PendingNodes<'a>>,
    pub next_index: usize,
}

/// Sub errors, in tree order. Excludes the root and native sources.
#[derive(Clone)]
#[must_use]
pub struct ErNodes<'a> {
    pub nodes: slice::Iter<'a, ErNode>,
    pub pending: Vec<slice::Iter<'a, ErNode>>,
    /// The shared trees already walked, set when each one should only be walked once.
    pub seen: Option<BTreeSet<usize>>,
}

/// Walk a native `Error::source()` chain. Starts with [`MAX_SOURCE_HOPS`] hops.
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
    pub nodes: ErNodes<'a>,
    pub marker: PhantomData<fn() -> T>,
}
