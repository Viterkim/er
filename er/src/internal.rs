//! er's plumbing: how it prints, and the markers and traits that let `.er()` and the macros pick the right impl.
//! Generated code and er's own impls use these, nothing else should.
use crate::{
    ErErrorWithSaved, ErFailure, ErNode, ErPart, ErSaved, ErSavedList, ErSnapshotEntry,
    ErSrcLocation, ErTree,
};
use alloc::string::String;
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
            #[cfg(feature = "src_locations")]
            src_location: Some(tree.src_location),
            #[cfg(not(feature = "src_locations"))]
            src_location: None,
        }
    }
}

/// Prints the whole saved report.
pub struct InternalSnapshotReportRef<'a> {
    pub entries: &'a [ErSnapshotEntry],
    pub saved: &'a [ErSaved],
    /// Reports saved on the Result holding the snapshot, printed after the snapshot's own.
    pub extra: &'a [ErSaved],
}

/// How `.er()` and the aggregation macros make the top error.
#[diagnostic::on_unimplemented(
    message = "can't build the top error from `{Self}`",
    label = "give er the fields or a closure that builds the error",
    note = "use `.er(())` for an empty struct or `.er(path)` to turn a `&Path` into a `PathBuf` on failure",
    note = "use `|_| fields` to compute or move fields on failure, or `|| error` for the whole error"
)]
#[doc(hidden)]
pub trait ErMake<A, Mode> {
    #[track_caller]
    fn er_make(self) -> ErErrorWithSaved<A>;
}

/// What `.er_save()` takes.
#[cfg_attr(
    feature = "stack_traces",
    diagnostic::on_unimplemented(
        message = "`.er_save()` can't save a `{Self}`",
        label = "it saves text, a snapshot or a backtrace",
        note = "`.er_save()` is for someone else's report. Data you want to read later goes on your error as a field"
    )
)]
#[cfg_attr(
    not(feature = "stack_traces"),
    diagnostic::on_unimplemented(
        message = "`.er_save()` can't save a `{Self}`",
        label = "it saves text or a snapshot",
        note = "a `Backtrace` needs er's `stack_traces` feature",
        note = "`.er_save()` is for someone else's report. Data you want to read later goes on your error as a field"
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
/// `Form` is [`ErFields`] when you supply every field or `ErDefaults` when you omit the
/// `#[er(default)]` fields.
#[diagnostic::on_unimplemented(
    message = "`.er()` can't make a `{Self}` out of `{Fields}`",
    label = "these aren't the fields of `{Self}`",
    note = "give its fields in order, `.er(path)` for one, `.er((path, port))` for more, `.er(())` for none. An enum needs its variant, `.er(|| {Self}::some_variant(..))`",
    note = "the shorthand only knows errors from `#[derive(Er)]` on your er version. For anything else, like a thiserror type or one from a crate on another er version, make it yourself, `.er(|| {Self} {{ .. }})`"
)]
#[doc(hidden)]
pub trait ErFromFields<Fields, Form>: Sized {
    #[track_caller]
    fn er_from_fields(fields: Fields) -> Self;
}

pub struct ErDefaults;

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
pub trait ErFieldsFor<A, Form> {}

impl<A: Error> ErFieldsFor<A, ErWhole> for ErFieldInput<A> {}
impl ErFieldsFor<crate::ErFailed, crate::ErFields> for ErFieldInput<()> {}

#[doc(hidden)]
pub trait ErBuildFrom<Fields, Form>: Sized {
    #[track_caller]
    fn er_build_from(fields: Fields) -> Self;
}

#[doc(hidden)]
pub trait ErWithFields<E: ?Sized, A, Mode>: for<'a> FnOnce(&'a E) -> Self::Input {
    type Input;

    #[track_caller]
    fn er_with_fields(self, error: &E) -> ErErrorWithSaved<A>;
}

#[doc(hidden)]
#[cfg_attr(
    not(er_unsync),
    diagnostic::on_unimplemented(
        message = "the top of `{Self}` can't go below another error",
        label = "the top needs to fit in `Box<dyn Error + Send + Sync + 'static>`",
        note = "use er-unsync to keep a local error"
    )
)]
#[cfg_attr(
    er_unsync,
    diagnostic::on_unimplemented(
        message = "the top of `{Self}` can't go below another error",
        label = "the top needs to fit in `Box<dyn Error + 'static>`"
    )
)]
pub trait ErTreeContext<Mode> {
    type Fields;

    #[track_caller]
    fn with_top<A: Error + 'static>(self, error: ErErrorWithSaved<A>) -> ErTree<A>;
}

#[doc(hidden)]
pub trait ErValFields<E, A, Mode>: FnOnce(E) -> Self::Input {
    type Input;

    #[track_caller]
    fn er_val_fields(self, value: E) -> ErErrorWithSaved<A>;
}

#[doc(hidden)]
pub trait ErTestInput<Input = ()>: Sized {
    #[track_caller]
    fn into_er_test_failure(self) -> ErFailure;
}

// Test failures are inputs too, but IntoErPart would overlap their From<Self>.
#[diagnostic::on_unimplemented(
    message = "`{Self}` isn't an error, so er can't put it in the tree",
    label = "this has to be an error",
    note = "a plain value like `Err(85)` goes on your own error with `.er_val(|v| v)`, or give it a type with `#[derive(Er)]`"
)]
#[doc(hidden)]
pub trait ErInput<Input = ()> {
    type Error: ?Sized;

    fn er_input_error(&self) -> &Self::Error;

    #[track_caller]
    fn into_er_input(self) -> ErPart;
}

#[cfg(feature = "macros")]
pub struct ErBailRoot<P, Kind>(pub P, pub core::marker::PhantomData<Kind>);

#[cfg(feature = "macros")]
#[doc(hidden)]
pub trait ErBailTestTop<Mode, Tag = u8> {
    #[track_caller]
    fn into_test_part(self, tag: Tag) -> ErPart;
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
    label = "expected an error, a tree, a Result or an Option holding one of them",
    note = "a plain value like `Err(85)` goes on your own error first, `.er_val(|v| v)`"
)]
#[doc(hidden)]
pub trait ErAllItem<Mode> {
    type Ok;
    const ALWAYS_ERROR: bool = false;

    #[track_caller]
    fn er_all_item(self) -> Result<Self::Ok, ErPart>;
}

pub struct ErAllResult;
pub struct ErAllOption;

pub use crate::wrapped::ErWrappedItem;
#[cfg(feature = "macros")]
pub use crate::wrapped::{
    ErAggregateAdd, ErAggregateItem, ErAssertInput, ErBailValue, ErMacroItem, ErWrappedFallback,
};

pub struct ErTestUnit;
