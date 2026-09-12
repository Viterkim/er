use crate::types_helpers::Pending;
use alloc::{boxed::Box, string::String, vec::Vec};
use core::{error::Error, fmt, panic::Location};

/// A Result with your typed error on top.
pub type Er<T, E> = Result<T, ErTree<E>>;

/// Where an error entered the tree.
pub type SrcLocation = &'static Location<'static>;

/// Your top error and the errors below, the typed one is in `tree.top`.
#[must_use]
pub struct ErTree<E> {
    pub top: E,
    pub nodes: Vec<ErNode>,
    #[cfg(feature = "src_locations")]
    pub src_location: SrcLocation,
}

/// A boxed error that can go in the tree.
pub type BoxError = Box<dyn Error + Send + Sync + 'static>;

/// One stored error and its children.
#[must_use]
pub struct ErNode {
    pub error: BoxError,
    pub nodes: Vec<ErNode>,
    #[cfg(feature = "src_locations")]
    pub src_location: SrcLocation,
}

/// Just the outer top error, owns the tree.
#[must_use]
pub struct ErTop<E> {
    pub tree: ErTree<E>,
    pub layout: Layout,
}

/// Just the outer error, borrows the tree.
#[must_use]
pub struct ErTopRef<'a, E> {
    pub tree: &'a ErTree<E>,
    pub layout: Layout,
}

/// The whole report, owns the tree.
#[must_use]
pub struct ErReport<E> {
    pub tree: ErTree<E>,
    pub layout: Layout,
}

/// The whole report, borrows the tree.
#[must_use]
pub struct ErReportRef<'a, E> {
    pub tree: &'a ErTree<E>,
    pub layout: Layout,
}

/// An opaque standard Error. The presentation is still in `.0`.
/// `source()` is empty, so ordinary error searches cannot see its tree.
#[must_use]
pub struct ErAsError<P>(pub P);

/// Layout variant for reports and snapshots
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Layout {
    #[default]
    Multiline,
    SingleLine,
}

/// Kind of entry in the tree
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

/// Root, native sources, then nodes.
/// Longer native chains stop at [`crate::walk::MAX_SOURCE_HOPS`], setting the last entry's `source_truncated`.
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

/// Up to [`crate::walk::MAX_SOURCE_HOPS`] errors in a native `Error::source()` chain.
#[must_use]
pub struct ErSources<'a> {
    pub next: Option<&'a (dyn Error + 'static)>,
    pub remaining: usize,
    /// Set when iteration stops at the limit with another source left.
    pub truncated: bool,
}

/// Saved messages and locations, no original error values.
#[derive(Clone)]
#[must_use]
pub struct ErSnapshot {
    /// Tree order, with parent indices pointing into this list.
    /// Don't reorder these unless you also fix everything that says where they belong in the tree.
    pub entries: Vec<ErSnapshotEntry>,
}

/// One saved error, not one printed line.
#[derive(Clone)]
pub struct ErSnapshotEntry {
    pub message: String,
    pub kind: ErEntryKind,
    pub index: usize,
    pub parent: Option<usize>,
    pub depth: usize,
    pub is_last: bool,
    /// The native source walk stopped here, the saved tree is incomplete.
    pub source_truncated: bool,
    #[cfg(feature = "src_locations")]
    pub src_location: Option<ErSnapshotLocation>,
}

/// File, line, column.
#[derive(Clone, PartialEq, Eq)]
pub struct ErSnapshotLocation {
    pub file: String,
    pub line: u32,
    pub column: u32,
}

/// The whole saved report, borrows the snapshot.
#[derive(Clone, Copy)]
#[must_use]
pub struct ErSnapshotReport<'a> {
    pub snapshot: &'a ErSnapshot,
    pub layout: Layout,
}

/// Just the saved outer error, borrows the snapshot.
#[derive(Clone, Copy)]
#[must_use]
pub struct ErSnapshotTop<'a> {
    pub snapshot: &'a ErSnapshot,
    pub layout: Layout,
}

/// Formatting or callback failure.
#[derive(Debug, PartialEq, Eq)]
pub enum LineError<E> {
    Format(fmt::Error),
    Callback(E),
}
