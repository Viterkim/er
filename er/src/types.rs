use alloc::{boxed::Box, vec::Vec};
use core::{error::Error, panic::Location};

/// A Result with your typed error on top.
pub type ErResult<T, E> = Result<T, ErTree<E>>;

/// Where an error entered the tree.
pub type SrcLocation = &'static Location<'static>;

/// Root 0, then each child and its descendants. Native sources don't count.
/// IDs belong to the current tree, wrapping updates the ones stored in its traces.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ErErrorId(pub usize);

/// Your top error and the errors below, the typed one is in `tree.top`.
#[must_use]
pub struct ErTree<E> {
    pub top: E,
    pub nodes: Vec<ErNode>,
    #[cfg(feature = "stack_traces")]
    pub stack_traces: Vec<ErStackTrace>,
    #[cfg(feature = "src_locations")]
    pub src_location: SrcLocation,
}

/// A boxed error that can go in the tree.
pub type BoxError = Box<dyn Error + Send + Sync + 'static>;

/// One stored error and the sub errors below it.
#[must_use]
pub struct ErNode {
    pub error: BoxError,
    pub nodes: Vec<ErNode>,
    #[cfg(feature = "src_locations")]
    pub src_location: SrcLocation,
}

/// An erased subtree with its stack traces, ready to go under another error.
#[must_use]
pub struct ErPart {
    pub node: ErNode,
    #[cfg(feature = "stack_traces")]
    pub stack_traces: Vec<ErStackTrace>,
}

#[cfg(feature = "stack_traces")]
pub struct ErStackTrace {
    pub error_name: &'static str,
    pub trace_location: SrcLocation,
    pub error_id: ErErrorId,
    pub capture: Box<std::backtrace::Backtrace>,
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
#[derive(Clone, Copy, PartialEq, Eq)]
#[must_use]
pub struct ErAsError<P>(pub P);

/// Layout variant for reports and snapshots
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Layout {
    #[default]
    Multiline,
    SingleLine,
}
