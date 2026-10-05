#[cfg(er_unsync)]
use alloc::rc::Rc as Shared;
#[cfg(all(not(er_unsync), target_has_atomic = "ptr"))]
use alloc::sync::Arc as Shared;
use alloc::{boxed::Box, vec::Vec};
use core::{error::Error, panic::Location};

/// A Result with your typed error on top.
pub type ErResult<T, E> = Result<T, ErTree<E>>;

/// Share data between errors by passing `&old.field` to the next constructor.
#[cfg_attr(not(er_unsync), doc = "Uses Arc, so the target needs pointer atomics.")]
#[cfg_attr(er_unsync, doc = "Uses Rc.")]
#[cfg(any(er_unsync, target_has_atomic = "ptr"))]
pub struct ErShared<T: ?Sized> {
    pub value: Shared<T>,
}

/// Where an error entered the tree.
pub type SrcLocation = &'static Location<'static>;

/// Root 0, then each child and its descendants. Native sources don't count.
/// Indices belong to the current tree, wrapping updates the ones stored in its traces.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ErErrorIndex(pub usize);

/// Your top error and the errors below, the typed one is in `tree.top`.
#[must_use]
pub struct ErTree<E> {
    pub top: E,
    /// If you reorder or remove nodes yourself, update the trace indices too.
    pub nodes: Vec<ErNode>,
    #[cfg(feature = "stack_traces")]
    pub stack_traces: Vec<ErStackTrace>,
    #[cfg(feature = "src_locations")]
    pub src_location: SrcLocation,
}

/// A boxed error that can go in the tree.
#[cfg(not(er_unsync))]
pub type BoxError = Box<dyn Error + Send + Sync + 'static>;

/// A boxed error that can go in the tree.
#[cfg(er_unsync)]
pub type BoxError = Box<dyn Error + 'static>;

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

/// A captured stack, linked to its error by `error_index`.
#[cfg(feature = "stack_traces")]
pub struct ErStackTrace {
    pub error_name: &'static str,
    pub trace_location: SrcLocation,
    pub error_index: ErErrorIndex,
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

#[cfg(feature = "macros")]
#[doc(hidden)]
pub struct ErAllResult;

#[cfg(feature = "macros")]
#[doc(hidden)]
pub struct ErAllError;
