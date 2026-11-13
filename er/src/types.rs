use crate::ErSnapshot;
#[cfg(all(not(er_unsync), not(target_has_atomic = "ptr")))]
pub(crate) use alloc::boxed::Box as Shared;
#[cfg(er_unsync)]
pub(crate) use alloc::rc::Rc as Shared;
#[cfg(all(not(er_unsync), target_has_atomic = "ptr"))]
pub(crate) use alloc::sync::Arc as Shared;
use alloc::{boxed::Box, string::String, vec::Vec};
use core::{error::Error, panic::Location};
#[cfg(feature = "stack_traces")]
use std::{backtrace::Backtrace, sync::Arc};

/// A Result with your typed error on top.
pub type ErResult<T, E> = Result<T, ErTree<E>>;

/// Share data between errors without copying the value.
#[cfg_attr(not(er_unsync), doc = "Uses Arc, so the target needs pointer atomics.")]
#[cfg_attr(er_unsync, doc = "Uses Rc.")]
#[cfg(any(er_unsync, target_has_atomic = "ptr"))]
pub struct ErShared<T: ?Sized> {
    pub value: Shared<T>,
}

/// A finished tree you can clone and put below other errors with `.er()`, `er_add!` or `er_all!`.
/// To add more errors after sharing it, build a new tree on top.
#[cfg_attr(not(er_unsync), doc = "Uses Arc, so the target needs pointer atomics.")]
#[cfg_attr(er_unsync, doc = "Uses Rc.")]
#[cfg(any(er_unsync, target_has_atomic = "ptr"))]
#[must_use]
pub struct ErSharedTree<E> {
    pub tree: Shared<ErTree<E>>,
}

/// Where an error entered the tree.
pub type ErSrcLocation = &'static Location<'static>;

/// Index 0 is the root. Count each sub error and everything below it in tree order.
/// Skip native sources. Indices belong to the current tree.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ErErrorIndex(pub usize);

/// Your top error and the errors below, the typed one is in `tree.top`.
///
/// When you want to panic on failure, call [`.er_unwrap_report()`](crate::ErPresentationExt::er_unwrap_report)
/// or [`.er_expect_report(..)`](crate::ErPresentationExt::er_expect_report) on the Result.
#[must_use]
pub struct ErTree<E> {
    pub top: E,
    pub nodes: Vec<ErNode>,
    pub saved: ErSavedList,
    #[cfg(feature = "src_locations")]
    pub src_location: ErSrcLocation,
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
    /// Leave this empty on a node that holds a shared tree, its errors live inside the shared tree and
    /// pushing here hides them. Build a new tree on top instead.
    pub nodes: Vec<ErNode>,
    #[cfg(feature = "src_locations")]
    pub src_location: ErSrcLocation,
}

/// An erased subtree with its saved reports, ready to go under another error.
#[must_use]
pub struct ErPart {
    pub node: ErNode,
    pub saved: ErSavedList,
}

/// A report you saved with `.er_save()`, printed below the tree under its title.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ErSaved {
    pub title: String,
    pub kind: ErSavedKind,
}

/// An error or fields with a report saved on them, `er_bail!(SyncErr::server(code).er_save("server", report))`.
#[must_use]
pub struct ErErrorWithSaved<T> {
    pub value: T,
    pub saved: ErSavedList,
}

/// Everything saved with `.er_save()`. Loop over it or index it like a slice, `for saved in &tree.saved`.
#[derive(Clone, Debug, Default)]
pub struct ErSavedList(pub Option<Shared<Vec<ErSaved>>>);

/// What got saved.
#[derive(Clone, Debug)]
pub enum ErSavedKind {
    Text(String),
    Snapshot(ErSnapshot),
    #[cfg(feature = "stack_traces")]
    StackTrace(Arc<Backtrace>),
}
