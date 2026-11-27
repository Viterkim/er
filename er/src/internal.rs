//! er's plumbing: how it prints, and the markers and traits that let `.er()` and the macros pick the right impl.
//! Generated code and er's own impls use these, nothing else should.
#[cfg(feature = "macros")]
use crate::ErBranch;
use crate::{
    ErErrorWithSaved, ErNode, ErSaved, ErSavedList, ErSnapshotEntry, ErSrcLocation, ErTestFailure,
    ErTree,
};
use alloc::{string::String, vec::Vec};
use core::error::Error;

/// Prints a whole report without knowing the top's type, so every error type shares one copy of the printing code.
pub struct InternalReport<'a> {
    pub top: &'a (dyn Error + 'static),
    pub nodes: &'a [ErNode],
    pub saved: &'a ErSavedList,
    /// Reports saved on the Result holding the tree, printed after the tree's own.
    pub extra: &'a [ErSaved],
    pub src_location: Option<ErSrcLocation>,
}
impl<'a> InternalReport<'a> {
    #[inline]
    pub fn new<E: ErReportError<Input>, Input>(tree: &'a ErTree<E>) -> Self {
        Self {
            top: tree.top.as_error(),
            nodes: &tree.nodes,
            saved: &tree.saved,
            extra: &[],
            src_location: tree.src_location,
        }
    }
}

#[inline]
#[cfg_attr(feature = "src_locations", track_caller)]
pub(crate) fn caller_location() -> Option<ErSrcLocation> {
    #[cfg(feature = "src_locations")]
    return Some(core::panic::Location::caller());
    #[cfg(not(feature = "src_locations"))]
    None
}

/// Prints the whole saved report.
pub struct InternalSnapshotReportRef<'a> {
    pub entries: &'a [ErSnapshotEntry],
    pub unattached: &'a [ErSaved],
    /// Reports saved on the Result holding the snapshot, printed after the snapshot's own.
    pub extra: &'a [ErSaved],
}

/// How `.er()` and the aggregation macros make the top error.
#[diagnostic::on_unimplemented(
    message = "can't build `{A}` from `{Self}`",
    label = "can't construct the error from this input",
    note = "one value per field, in order, `#[er(default)]` ones at the end can be left out and `#[er(src_location)]` fills itself in",
    note = "fields only go in directly when they're all `Copy + Debug`, like a `&Path`, a number or `()`, a `String` or an `ErShared` isn't, not even inside an `Option`",
    note = "anything else goes in a closure, `|_| (path, user.name())`, it only runs on failure",
    note = "a `pub` struct with private fields has no tuple shorthand. Drop `pub` if the type is module-local, or return a finished error, `|| MyErr::new(..)`"
)]
#[doc(hidden)]
pub trait ErMake<A, Mode> {
    type Nodes: Into<Vec<ErNode>>;

    /// Build the error in a cold call. Saving a report always goes there, building it in every caller slowed
    /// down the calls that succeed. Everything else goes cold only when the error has drop glue, like a String
    /// field. Plain data built in place measured 10% faster on failure and no slower on success.
    const COLD: bool = true;

    #[track_caller]
    fn er_make(self) -> ErErrorWithSaved<A, Self::Nodes>;
}

/// Whether `.er()` should build this error in a cold call, see `ErMake::COLD`.
pub fn er_make_cold<A, Mode, M: ErMake<A, Mode>>(_: &M) -> bool {
    M::COLD
}

/// What `.er_save()` takes.
#[cfg_attr(
    feature = "stack_traces",
    diagnostic::on_unimplemented(
        message = "`.er_save()` can't save a `{Self}`",
        label = "it saves text, a snapshot or a backtrace",
        note = "`.er_save()` is for stashing someone else's report, data you want to use later goes on your error as a field"
    )
)]
#[cfg_attr(
    not(feature = "stack_traces"),
    diagnostic::on_unimplemented(
        message = "`.er_save()` can't save a `{Self}`",
        label = "it saves text or a snapshot",
        note = "a `Backtrace` needs er's `stack_traces` feature on in Cargo.toml",
        note = "`.er_save()` is for stashing someone else's report, data you want to use later goes on your error as a field"
    )
)]
#[doc(hidden)]
pub trait ErSaveInput {
    fn er_save_into(self, title: String, saved: &mut ErSavedList);
}

/// What `.er_save()` on a Result can borrow from the error.
#[diagnostic::on_unimplemented(
    message = "`.er_save()` can't save a `{Self}`",
    label = "borrow the report or snapshot from the error, like `|err| &err.body`"
)]
#[doc(hidden)]
pub trait ErSaveRef {
    fn er_save_ref(&self, title: String, saved: &mut ErSavedList);
}

pub struct ErReady;

/// Build the new top error from the fields returned by the closure.
pub struct ErFields;

/// `#[derive(Er)]` implements this so `.er(|_| fields)` can build an error from the closure's fields.
/// `Form` is [`ErFields`] when you supply every field or `ErDefaults<GIVEN>` when you leave out
/// `#[er(default)]` fields from the end, `GIVEN` is how many of them you did give.
#[diagnostic::on_unimplemented(
    message = "can't construct `{Self}` from `{Fields}` using field shorthand",
    label = "no matching field shorthand for this input",
    note = "pass struct fields in order, like `()`, `path` or `(path, port)`. For an enum, build its variant in the closure",
    note = "field shorthand needs constructors generated by the same version of er",
    note = "a `pub` struct with private fields has no tuple shorthand. Drop `pub` if the type is module-local, or return a finished error, `|| {Self} {{ .. }}`"
)]
#[doc(hidden)]
pub trait ErFromFields<Fields, Form = ErFields>: Sized {
    #[track_caller]
    fn er_from_fields(fields: Fields) -> Self;
}

pub struct ErDefaults<const GIVEN: usize>;

pub struct ErWhole;

pub struct ErWithSaves;

pub struct ErSavedTree;

pub struct ErSavedSnapshot;

pub struct ErBoxedInput;

/// Borrow the error at the top of a tree or beside a report.
pub trait ErReportError<Input>: 'static {
    fn as_error(&self) -> &(dyn Error + 'static);
}

pub struct ErWithReport<Input = ()>(core::marker::PhantomData<Input>);

// The runtime knows whether sharing exists, including er-unsync on targets without atomics.
#[cfg(any(er_unsync, target_has_atomic = "ptr"))]
#[doc(hidden)]
#[macro_export]
macro_rules! __er_if_shared {
    ($($item:item)*) => { $($item)* };
}

#[cfg(not(any(er_unsync, target_has_atomic = "ptr")))]
#[doc(hidden)]
#[macro_export]
macro_rules! __er_if_shared {
    ($($item:item)*) => {};
}

/// What can sit next to an error as its report, a `String` or an `Option<String>`.
#[doc(hidden)]
pub trait ErReportText: ErSaveInput {
    fn er_report_str(&self) -> Option<&str>;

    fn into_er_report_string(self) -> Option<String>;
}

#[doc(hidden)]
pub trait ErForm {}

#[doc(hidden)]
pub trait ErValueForm {}

// Start inference from the value a closure returned. Looking up an unknown error type
// by its fields alone leaves Rust guessing even when the value is already a finished error.
pub struct ErFieldInput<P>(core::marker::PhantomData<P>);
#[doc(hidden)]
pub trait ErFieldsFor<A, Form = ErFields> {}

impl<A: Error> ErFieldsFor<A, ErWhole> for ErFieldInput<A> {}
impl ErFieldsFor<crate::ErTestError, crate::ErFields> for ErFieldInput<()> {}
impl ErFieldsFor<crate::ErMainError, crate::ErFields> for ErFieldInput<()> {}

#[doc(hidden)]
pub trait ErBuildFrom<Fields, Form>: Sized {
    #[track_caller]
    fn er_build_from(fields: Fields) -> Self;
}

#[doc(hidden)]
#[diagnostic::on_unimplemented(
    message = "can't construct `{A}` from this closure",
    label = "the closure takes `&{E}` and returns fields or a finished error",
    note = "a `pub` struct with private fields has no tuple shorthand. Drop `pub` if the type is module-local, or build the error in the closure"
)]
pub trait ErWithFields<E: ?Sized, A, Mode>: for<'a> FnOnce(&'a E) -> Self::Input {
    type Nodes: Into<Vec<ErNode>>;

    type Input;

    #[track_caller]
    fn er_with_fields(self, error: &E) -> ErErrorWithSaved<A, Self::Nodes>;
}

#[doc(hidden)]
#[cfg_attr(
    not(er_unsync),
    diagnostic::on_unimplemented(
        message = "the top of `{Self}` can't go below another error",
        label = "needs to own its data and be safe across threads",
        note = "if it holds an `Rc` or a `RefCell`, use er-unsync, it keeps errors on one thread",
        note = "if it borrows something like a `&str`, store a `String` or a `&'static str` instead"
    )
)]
#[cfg_attr(
    er_unsync,
    diagnostic::on_unimplemented(
        message = "the top of `{Self}` can't go below another error",
        label = "has to own its data",
        note = "if it borrows something like a `&str`, store a `String` or a `&'static str` instead"
    )
)]
pub trait ErTreeContext<Mode> {
    type Fields;

    #[track_caller]
    fn with_top<A: Error + 'static>(
        self,
        error: ErErrorWithSaved<A, impl Into<Vec<ErNode>>>,
    ) -> ErTree<A>;
}

#[doc(hidden)]
#[diagnostic::on_unimplemented(
    message = "can't construct `{A}` from this closure",
    label = "the closure takes `{E}` and returns fields or a finished error",
    note = "a `pub` struct with private fields has no tuple shorthand. Drop `pub` if the type is module-local, or build the error in the closure"
)]
pub trait ErValFields<E, A, Mode>: FnOnce(E) -> Self::Input {
    type Nodes: Into<Vec<ErNode>>;

    type Input;

    #[track_caller]
    fn er_val_fields(self, value: E) -> ErErrorWithSaved<A, Self::Nodes>;
}

#[doc(hidden)]
pub trait ErTestInput<Input = ()>: Sized {
    #[track_caller]
    fn into_er_test_failure(self) -> ErTestFailure;
}

// Test failures are inputs too, but IntoErNode would overlap their From<Self>.
#[diagnostic::on_unimplemented(
    message = "`{Self}` isn't an error, so er can't put it in the tree",
    label = "this has to be an error",
    note = "put a plain value like `Err(85)` on your own error with `.er_val(|v| v)`, or make it an error with `#[derive(Er)]`"
)]
#[doc(hidden)]
pub trait ErInput<Input = ()> {
    type Error: ?Sized;

    fn er_input_error(&self) -> &Self::Error;

    #[track_caller]
    fn into_er_input(self) -> ErNode;
}

#[cfg(feature = "macros")]
pub struct ErBailRoot<P, Kind>(pub P, pub core::marker::PhantomData<Kind>);

#[cfg(feature = "macros")]
#[doc(hidden)]
pub trait ErBailTestTop<Mode, Tag = u8> {
    #[track_caller]
    fn into_test_node(self, tag: Tag) -> ErNode;
}

#[cfg(feature = "macros")]
#[doc(hidden)]
pub trait ErBailNodes<P, T, Mode>: Sized {
    type Top;

    #[track_caller]
    fn prepare(self, top: P) -> (core::marker::PhantomData<T>, ErTree<Self::Top>);
}

#[cfg(feature = "macros")]
#[doc(hidden)]
pub trait ErBailTree<T> {
    #[track_caller]
    fn finish(self, target: core::marker::PhantomData<T>) -> T;
}

/// What `er_bail!` can hand back, a tree, a Wrap or the `(Error, String)` pair, and the error it builds for it.
#[cfg(feature = "macros")]
#[doc(hidden)]
pub trait ErBailTarget {
    type Error;
}
#[cfg(feature = "macros")]
impl<T: crate::IntoErTree> ErBailTarget for T {
    type Error = T::Error;
}
#[cfg(feature = "macros")]
impl<E: Error + 'static> ErBailTarget for (E, alloc::string::String) {
    type Error = E;
}

#[cfg(feature = "macros")]
#[doc(hidden)]
pub trait ErBail<T, Mode> {
    #[track_caller]
    fn er_bail(self) -> T;
}

#[cfg(feature = "macros")]
#[doc(hidden)]
pub trait ErBailFields<T, Mode> {
    #[track_caller]
    fn er_bail_fields(self) -> T;
}

#[cfg(feature = "macros")]
#[inline]
// FnOnce lets the closure update a captured value and return a borrow of it.
pub fn factory<P>(make: impl FnOnce(()) -> P) -> impl FnOnce(()) -> P {
    make
}

#[cfg(feature = "macros")]
#[inline]
pub fn ready_factory<P>(make: impl FnOnce() -> P) -> impl FnOnce() -> P {
    make
}

#[cfg(feature = "macros")]
// Keep method lookup on this version of Er when the caller has two versions in scope.
pub struct ErNew<T>(pub T);

#[cfg(feature = "macros")]
#[doc(hidden)]
pub trait ErNewPass<P>: Sized {
    fn __er_add_pending(self) -> Self {
        self
    }

    fn __er_bail_value(self) -> P;

    fn __er_tree(self) -> Self {
        self
    }

    fn __er_factory(self) -> Self {
        self
    }

    fn __er_bail_root(self) -> ErBailRoot<P, ErFields>;
}

#[cfg(feature = "macros")]
#[doc(hidden)]
pub struct ErPrepared<T>(pub T);

#[cfg(feature = "macros")]
#[doc(hidden)]
pub trait ErAddReady<Input> {
    type Error;

    fn __er_add_ready(self) -> ErBranch<Self::Error>;
}

#[cfg(feature = "macros")]
#[doc(hidden)]
pub trait ErReadyTree {
    type Top;

    #[track_caller]
    fn into_ready_tree(self) -> ErTree<Self::Top>;
}

#[cfg(feature = "macros")]
#[doc(hidden)]
pub trait ErNewFields<E, Form> {
    #[track_caller]
    fn __er_finish(self) -> ErTree<E>;
}

#[diagnostic::on_unimplemented(
    message = "can't add a `{Self}` as a sub error",
    label = "needs an error, a tree, or a Result or Option of one",
    note = "turn a plain value like `Err(85)` into your own error first with `.er_val(|v| v)`"
)]
#[doc(hidden)]
pub trait ErAllItem<Mode> {
    type Ok;
    const ALWAYS_ERROR: bool = false;

    #[track_caller]
    fn er_all_item(self) -> Result<Self::Ok, ErNode>;
}

pub struct ErAllResult;
pub struct ErAllOption;

pub use crate::wrapped::ErWrappedItem;
#[cfg(feature = "macros")]
pub use crate::wrapped::{
    ErAggregateAdd, ErAggregateItem, ErAssertInput, ErBailValue, ErMacroItem, ErWrappedFallback,
};

pub struct ErTestUnit;
