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

/// Finish a closure or async block with an `ErResult`. Use `ErOk::<MyErr, _>(value)` to name its error.
#[allow(non_snake_case)]
pub const fn ErOk<E, T>(value: T) -> ErResult<T, E> {
    Ok(value)
}

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

/// Your top error and the errors below, the typed one is in `tree.top`.
///
/// When you want to panic on failure, call [`.er_unwrap()`](crate::ErPresentationExt::er_unwrap)
/// or [`.er_expect(..)`](crate::ErPresentationExt::er_expect) on the Result.
#[must_use]
pub struct ErTree<E> {
    pub top: E,
    pub nodes: Vec<ErNode>,
    /// Where the top was made, when a location was captured or supplied.
    pub src_location: Option<ErSrcLocation>,
    /// Reports saved on the top error. Use [`er_saved()`](Self::er_saved) for the whole tree.
    pub saved: ErSavedList,
}

/// A boxed error that can go in the tree.
#[cfg(not(er_unsync))]
pub type ErBoxError = Box<dyn Error + Send + Sync + 'static>;

/// A boxed error that can go in the tree.
#[cfg(er_unsync)]
pub type ErBoxError = Box<dyn Error + 'static>;

/// One stored error and the sub errors below it.
#[must_use]
pub struct ErNode {
    pub error: ErBoxError,
    /// Reports saved on this error.
    pub saved: ErSavedList,
    /// Leave this empty on a node that holds a shared tree, its errors live inside the shared tree and
    /// pushing here hides them. Build a new tree on top instead.
    pub nodes: Vec<ErNode>,
    /// The location carried by this error, if any.
    pub src_location: Option<ErSrcLocation>,
}

/// A report you saved with `.er_save()`, printed under the error that saved it.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ErSaved {
    pub title: String,
    pub kind: ErSavedKind,
}

/// An error or fields with a report saved on them, `er_bail!(SyncErr::server(code).er_save("server", report))`.
#[must_use]
pub struct ErErrorWithSaved<T, Nodes = [ErNode; 0]> {
    pub value: T,
    pub saved: ErSavedList,
    pub nodes: Nodes,
}

/// An error with its children and saved reports, ready to add to another error or return as a failure.
pub type ErBranch<E> = ErErrorWithSaved<E, Vec<ErNode>>;

/// The attachments on one error. Loop over them or index them like a slice.
#[derive(Clone, Debug, Default)]
pub struct ErSavedList(pub Option<Shared<Vec<ErSaved>>>);

/// What got saved with `.er_save()`.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum ErSavedKind {
    Text(String),
    Snapshot(ErSnapshot),
    #[cfg(feature = "stack_traces")]
    StackTrace(Arc<Backtrace>),
}
