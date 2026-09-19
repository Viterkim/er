use alloc::{boxed::Box, vec::Vec};
use core::{error::Error, panic::Location};

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
