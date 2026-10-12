use crate::{
    BoxError, ErNode, ErPart, ErReport, ErReportRef, ErResult, ErSnapshot, ErTop, ErTopRef, ErTree,
};
#[cfg(feature = "test")]
use crate::{ErTest, ErTestFailure};
use alloc::string::String;
use core::error::Error;

/// A finished error for `.er_with()`, `.er_val()`, `.er_into()` and the tree and Wrap forms.
pub struct ErBuilt<E>(pub E);

/// Use this error directly instead of building it from fields.
pub const fn er_built<E>(error: E) -> ErBuilt<E> {
    ErBuilt(error)
}

#[doc(hidden)]
pub struct ErReady;

/// Build the new top error from the fields returned by the closure.
pub struct ErFields;

#[doc(hidden)]
pub struct ErSaved;

#[cfg(er_unsync)]
#[doc(hidden)]
pub struct ErBoxedInput;

/// How `.er()` makes its new top error.
pub trait ErMake<A, Mode> {
    fn er_make(self) -> A;
}

#[doc(hidden)]
pub trait ErWithFields<E: ?Sized, A, Mode>: for<'a> FnOnce(&'a E) -> Self::Input {
    type Input;

    fn er_with_fields(self, error: &E) -> A;
}

#[doc(hidden)]
pub trait ErValFields<E, A, Mode>: FnOnce(E) -> Self::Input {
    type Input;

    fn er_val_fields(self, value: E) -> A;
}

/// Add a new top error above a raw error.
pub trait ErErrorContextExt<Mode, Input = ()>: IntoErPart<Input> + Sized {
    /// Put your error above this one, keeping the original below.
    #[cfg_attr(feature = "src_locations", track_caller)]
    fn er<A>(self, error: impl ErMake<A, Mode>) -> ErTree<A>
    where
        A: Error + 'static,
    {
        ErTree::new(error.er_make(), [self])
    }
}

/// Build context using the old error.
pub trait ErErrorExt<Input = ()>: IntoErPart<Input> + Sized {
    /// Build the new top from fields taken from this error.
    ///
    /// `er_bail!(device.er_with(|err| err.code));`
    #[cfg_attr(feature = "src_locations", track_caller)]
    fn er_with<A, Mode>(self, fields: impl ErWithFields<Self, A, Mode>) -> ErTree<A>
    where
        A: Error + 'static,
    {
        let top = fields.er_with_fields(&self);
        ErTree::new(top, [self])
    }
}

/// Print or save a plain error and its native sources.
pub trait ErErrorPresentationExt {
    /// The whole report as text.
    fn er_report_string(&self) -> String
    where
        Self: 'static;

    /// Just this error as text.
    fn er_top_string(&self) -> String;

    /// Save the messages and source structure.
    fn er_snapshot(&self) -> ErSnapshot
    where
        Self: 'static;
}

/// Add context to a Result or turn None into an error.
pub trait ErContextExt<Mode, Input = ()> {
    type Ok;

    /// Add your error on the top, move everything else below it.
    /// Only happens on Err or None.
    ///
    /// ```rust,ignore
    /// result.er(())?; // Empty struct
    /// result.er(|_| path)?; // One field
    /// result.er(|_| (machine, token))?; // More fields
    /// result.er(|| EnumErr::variant_name(arg1))?; // Enum variant
    /// ```
    #[cfg_attr(feature = "src_locations", track_caller)]
    fn er<A>(self, error: impl ErMake<A, Mode>) -> ErResult<Self::Ok, A>
    where
        A: Error + 'static;
}

/// Add a new top error above an existing tree.
pub trait ErTreeContextExt<Mode> {
    /// Add your error on top, keeping the old tree below it.
    #[cfg_attr(feature = "src_locations", track_caller)]
    fn er<A>(self, error: impl ErMake<A, Mode>) -> ErTree<A>
    where
        A: Error + 'static;
}

/// Other ways to work with a Result's error.
pub trait ErResultExt<Input = ()> {
    type Ok;
    type Err;

    /// Build the new top from the old error. On an ErResult, you get its typed top.
    /// Return fields, an enum constructor, or `er_built(MyErr { ... })`.
    #[cfg_attr(feature = "src_locations", track_caller)]
    fn er_with<A, Mode>(
        self,
        fields: impl ErWithFields<<Self::Err as ErInput<Input>>::Error, A, Mode>,
    ) -> ErResult<Self::Ok, A>
    where
        Self: Sized,
        A: Error + 'static,
        Self::Err: ErInput<Input>,
    {
        self.er_build_tree(|source| fields.er_with_fields(source.er_input_error()))
    }

    /// Like `.er_with()`, but the closure gets the tree on an ErResult.
    #[cfg_attr(feature = "src_locations", track_caller)]
    fn er_with_tree<A, Mode>(
        self,
        fields: impl ErWithFields<Self::Err, A, Mode>,
    ) -> ErResult<Self::Ok, A>
    where
        Self: Sized,
        A: Error + 'static,
        Self::Err: ErInput<Input>,
    {
        self.er_build_tree(|source| fields.er_with_fields(source))
    }

    #[doc(hidden)]
    #[cfg_attr(feature = "src_locations", track_caller)]
    fn er_build_tree<A>(self, f: impl FnOnce(&Self::Err) -> A) -> ErResult<Self::Ok, A>
    where
        A: Error + 'static,
        Self::Err: ErInput<Input>;
}

/// Replace a Result's error with a value of your own.
pub trait ErValueExt {
    type Ok;
    type Err;

    /// For values that can't go in the tree, like `Err(85)`.
    /// Takes ownership of the value. Return fields, an enum constructor, or `er_built(MyErr { ... })`.
    ///
    /// ! WARNING ! Don't use this to add context to an error or tree, you'll nuke it. Use `.er()` or `.er_with()` for that.
    ///
    /// ```rust,ignore
    /// device_status().er_val(|v| v)?;
    /// ```
    #[cfg_attr(feature = "src_locations", track_caller)]
    fn er_val<A, Mode>(self, fields: impl ErValFields<Self::Err, A, Mode>) -> ErResult<Self::Ok, A>
    where
        Self: Sized,
        A: Error + 'static,
    {
        self.er_build_value(|v| fields.er_val_fields(v))
    }

    #[doc(hidden)]
    #[cfg_attr(feature = "src_locations", track_caller)]
    fn er_build_value<A>(self, f: impl FnOnce(Self::Err) -> A) -> ErResult<Self::Ok, A>
    where
        A: Error + 'static;
}

/// Collect successful values and add context to failures.
pub trait ErIteratorExt<Mode, Input = ()>: Sized {
    type Ok;

    /// Stop at the first error.
    #[cfg_attr(feature = "src_locations", track_caller)]
    fn er_collect<C, A>(self, error: impl ErMake<A, Mode>) -> ErResult<C, A>
    where
        C: FromIterator<Self::Ok>,
        A: Error + 'static;

    /// Keep the successful values until the iterator finishes. If anything failed,
    /// keep every error and drop the collected values.
    #[cfg_attr(feature = "src_locations", track_caller)]
    fn er_collect_all<C, A>(self, error: impl ErMake<A, Mode>) -> ErResult<C, A>
    where
        C: FromIterator<Self::Ok>,
        A: Error + 'static;
}

/// Get the tree or pick the output for a Result with a tree, Wrap or owned presentation.
pub trait ErPresentationExt {
    type Ok;
    type Err;

    /// Get the normal Er result back.
    fn er_tree(self) -> ErResult<Self::Ok, Self::Err>;

    /// Just the outer error, leaves Ok alone.
    ///
    /// `read_port("85").er_top()?;`
    fn er_top(self) -> Result<Self::Ok, ErTop<Self::Err>>;

    /// The whole report, leaves Ok alone.
    ///
    /// `read_port("85").er_report()?;`
    fn er_report(self) -> Result<Self::Ok, ErReport<Self::Err>>;

    /// Print the whole report, leaves Ok alone.
    fn er_report_string(self) -> Result<Self::Ok, String>
    where
        Self::Err: Error + 'static;

    /// Print just the outer error, leaves Ok alone.
    fn er_top_string(self) -> Result<Self::Ok, String>
    where
        Self::Err: core::fmt::Display;

    /// Save the failed tree as messages, leaves Ok alone.
    fn er_snapshot(self) -> Result<Self::Ok, ErSnapshot>
    where
        Self::Err: Error + 'static;

    /// Build your value from the failed report. Leaves Ok alone.
    /// Return fields, or the report itself to fill `into_*` fields automatically.
    /// Extra fields go beside the report: `|report| (report, (request_id, path))`.
    fn er_into<A, Mode>(
        self,
        fields: impl ErValFields<ErReport<Self::Err>, A, Mode>,
    ) -> Result<Self::Ok, A>
    where
        Self: Sized,
    {
        match self.er_report() {
            Ok(value) => Ok(value),
            Err(report) => Err(fields.er_val_fields(report)),
        }
    }

    /// Get the Ok value, or panic with the whole report.
    ///
    /// Same as `.er_report().unwrap()`.
    ///
    /// # Panics
    /// Panics if the result is Err.
    #[track_caller]
    fn unwrap_report(self) -> Self::Ok
    where
        Self: Sized,
        Self::Err: Error + 'static,
    {
        self.er_report().unwrap()
    }

    /// Get the Ok value, or panic with your message and the whole report.
    ///
    /// Same as `.er_report().expect(message)`.
    ///
    /// # Panics
    /// Panics if the result is Err.
    #[track_caller]
    fn expect_report(self, message: &str) -> Self::Ok
    where
        Self: Sized,
        Self::Err: Error + 'static,
    {
        self.er_report().expect(message)
    }

    /// Take the tree out of a Wrap with `std_error` and add context. Leaves Ok alone.
    ///
    /// !WARNING! `.er()` and `.er_with()` box a Wrap with `std_error`, so you can't find the errors inside it.
    #[cfg_attr(feature = "src_locations", track_caller)]
    fn er_wrap<A, Mode>(self, error: impl ErMake<A, Mode>) -> ErResult<Self::Ok, A>
    where
        Self: Sized,
        Self::Err: Error + Into<BoxError> + 'static,
        A: Error + 'static,
    {
        self.er_tree().er(error)
    }

    /// Take the tree out of a Wrap and build fields from its old top.
    #[cfg_attr(feature = "src_locations", track_caller)]
    fn er_with_wrap<A, Mode>(
        self,
        fields: impl ErWithFields<Self::Err, A, Mode>,
    ) -> ErResult<Self::Ok, A>
    where
        Self: Sized,
        Self::Err: Error + Into<BoxError> + 'static,
        A: Error + 'static,
    {
        self.er_tree().er_with(fields)
    }
}

/// Pick the output for a Result with a saved snapshot or its presentation.
pub trait ErSnapshotPresentationExt {
    type Ok;
    type Top: core::fmt::Display + core::fmt::Debug;
    type Report: core::fmt::Display + core::fmt::Debug;

    /// Just the saved outer error, leaves Ok alone.
    fn er_top(self) -> Result<Self::Ok, Self::Top>;

    /// The whole saved report, leaves Ok alone.
    fn er_report(self) -> Result<Self::Ok, Self::Report>;

    /// Print the whole saved report, leaves Ok alone.
    fn er_report_string(self) -> Result<Self::Ok, String>;

    /// Print just the saved outer error, leaves Ok alone.
    fn er_top_string(self) -> Result<Self::Ok, String>;

    /// Get the saved entries back, leaves Ok alone.
    fn er_snapshot(self) -> Result<Self::Ok, ErSnapshot>;

    /// Get the Ok value, or panic with the whole saved report.
    ///
    /// # Panics
    /// Panics if the result is Err.
    #[track_caller]
    fn unwrap_report(self) -> Self::Ok
    where
        Self: Sized,
    {
        self.er_report().unwrap()
    }

    /// Get the Ok value, or panic with your message and the whole saved report.
    ///
    /// # Panics
    /// Panics if the result is Err.
    #[track_caller]
    fn expect_report(self, message: &str) -> Self::Ok
    where
        Self: Sized,
    {
        self.er_report().expect(message)
    }
}

/// Pick the output for a Result that borrows a tree or its presentation.
pub trait ErBorrowedPresentationExt<'a> {
    type Ok;
    type Err: 'a;

    /// Get the borrowed tree back, leaves Ok alone.
    fn er_tree(self) -> Result<Self::Ok, &'a ErTree<Self::Err>>;

    /// Just the outer error, leaves Ok alone.
    fn er_top(self) -> Result<Self::Ok, ErTopRef<'a, Self::Err>>;

    /// The whole report, leaves Ok alone.
    fn er_report(self) -> Result<Self::Ok, ErReportRef<'a, Self::Err>>;

    /// Print the whole report, leaves Ok alone.
    fn er_report_string(self) -> Result<Self::Ok, String>
    where
        Self::Err: Error + 'static;

    /// Print just the outer error, leaves Ok alone.
    fn er_top_string(self) -> Result<Self::Ok, String>
    where
        Self::Err: core::fmt::Display;

    /// Save the failed tree as messages, leaves Ok alone.
    fn er_snapshot(self) -> Result<Self::Ok, ErSnapshot>
    where
        Self::Err: Error + 'static;

    /// Get the Ok value, or panic with the whole report.
    ///
    /// # Panics
    /// Panics if the result is Err.
    #[track_caller]
    fn unwrap_report(self) -> Self::Ok
    where
        Self: Sized,
        Self::Err: Error + 'static,
    {
        self.er_report().unwrap()
    }

    /// Get the Ok value, or panic with your message and the whole report.
    ///
    /// # Panics
    /// Panics if the result is Err.
    #[track_caller]
    fn expect_report(self, message: &str) -> Self::Ok
    where
        Self: Sized,
        Self::Err: Error + 'static,
    {
        self.er_report().expect(message)
    }
}

/// Borrow a tree or change its borrowed presentation, keeping an existing layout.
pub trait IntoErTreeRef<'a>: Sized {
    type Error: 'a;

    /// Borrow the whole report.
    fn into_er_report_ref(self) -> ErReportRef<'a, Self::Error>;

    /// Borrow just the outer error.
    fn into_er_top_ref(self) -> ErTopRef<'a, Self::Error> {
        self.into_er_report_ref().er_top()
    }

    /// Borrow the tree. Leaves the layout behind.
    fn into_er_tree_ref(self) -> &'a ErTree<Self::Error> {
        self.into_er_report_ref().tree
    }
}

/// Move a snapshot or change its presentation, keeping an existing layout.
pub trait IntoErSnapshot {
    type Top: core::fmt::Display + core::fmt::Debug;
    type Report: core::fmt::Display + core::fmt::Debug;

    /// Take the saved entries out. Borrowed snapshots copy their entries.
    fn into_er_snapshot(self) -> ErSnapshot;

    /// Just the saved outer error.
    fn into_er_top(self) -> Self::Top;

    /// The whole saved report.
    fn into_er_report(self) -> Self::Report;
}

/// Convert a tree into your own error, saving its diagnostics.
/// Generated by `#[derive(Er)]` for fields marked `into_top`,
/// `into_report_string` and `into_snapshot`.
/// Return the report from `.er_into()` to use these conversions automatically.
/// Foreign destinations can accept ordinary fields without implementing this trait.
pub trait ErFromTree<E, Input = ()>: Sized {
    /// Save the diagnostics and turn the tree into your error.
    fn er_from_tree(tree: ErTree<E>, input: Input) -> Self;

    /// Convert an owned report. Override this to use its chosen layout.
    /// The default delegates to `er_from_tree()`.
    fn er_from_report(report: ErReport<E>, input: Input) -> Self {
        Self::er_from_tree(report.tree, input)
    }
}

#[cfg(feature = "test")]
#[doc(hidden)]
pub trait ErTestInput<Input = ()>: Sized {
    #[cfg_attr(feature = "src_locations", track_caller)]
    fn into_er_test_failure(self) -> ErTestFailure;
}

/// Turn an Option or Result into ErTest. Existing test failures pass through.
#[cfg(feature = "test")]
pub trait ErTestExt<Input = ()> {
    type Ok;

    /// None reports "Option was None" with this call's location.
    /// `Err(())` reports a test failure at this call too.
    #[cfg_attr(feature = "src_locations", track_caller)]
    fn er_test(self) -> ErTest<Self::Ok>;
}

/// Give a presentation standard Error support. On a Result, leaves Ok alone.
pub trait ErOpaqueErrorExt {
    type Output;

    /// The presentation stays in `.0`, but error searches can't see inside it.
    fn opaque_err(self) -> Self::Output;
}

/// Turns an error or tree into a node and its metadata.
pub trait IntoErPart<Input = ()> {
    type Error: ?Sized;

    /// Borrow the error, or the top if this is a tree.
    fn er_error(&self) -> &Self::Error;

    /// Move the error and its traces into a subtree.
    #[cfg_attr(feature = "src_locations", track_caller)]
    fn into_er_part(self) -> ErPart;
}

// Test failures are inputs too, but IntoErPart would overlap their From<Self>.
#[doc(hidden)]
pub trait ErInput<Input = ()> {
    type Error: ?Sized;

    fn er_input_error(&self) -> &Self::Error;

    #[cfg_attr(feature = "src_locations", track_caller)]
    fn into_er_input(self) -> ErPart;
}

/// Get the tree or presentation out of a tree, Wrap or owned presentation.
pub trait IntoErTree {
    type Error;

    /// Take the tree out, keeping its errors and traces. Leaves the layout behind.
    fn into_er_tree(self) -> ErTree<Self::Error>;

    /// Add context after taking the tree out of its presentation or Wrap.
    #[cfg_attr(feature = "src_locations", track_caller)]
    fn er_wrap<A, Mode>(self, error: impl ErMake<A, Mode>) -> ErTree<A>
    where
        Self: Sized,
        Self::Error: Error + Into<BoxError> + 'static,
        A: Error + 'static,
    {
        self.into_er_tree().er(error)
    }

    /// Take the tree out and build fields from its old top.
    #[cfg_attr(feature = "src_locations", track_caller)]
    fn er_with_wrap<A, Mode>(self, fields: impl ErWithFields<Self::Error, A, Mode>) -> ErTree<A>
    where
        Self: Sized,
        Self::Error: Error + Into<BoxError> + 'static,
        A: Error + 'static,
    {
        self.into_er_tree().er_with(fields)
    }

    /// Build the new top from fields taken from the old top.
    #[cfg_attr(feature = "src_locations", track_caller)]
    fn er_with<A, Mode>(self, fields: impl ErWithFields<Self::Error, A, Mode>) -> ErTree<A>
    where
        Self: Sized,
        Self::Error: Error + Into<BoxError> + 'static,
        A: Error + 'static,
    {
        self.into_er_tree().er_with(fields)
    }

    /// Like `.er_with()`, but the closure gets the whole tree.
    #[cfg_attr(feature = "src_locations", track_caller)]
    fn er_with_tree<A, Mode>(
        self,
        fields: impl ErWithFields<ErTree<Self::Error>, A, Mode>,
    ) -> ErTree<A>
    where
        Self: Sized,
        Self::Error: Error + Into<BoxError> + 'static,
        A: Error + 'static,
    {
        self.into_er_tree().er_with_tree(fields)
    }

    /// Add errors below the current top.
    #[cfg_attr(feature = "src_locations", track_caller)]
    fn er_add<Input>(
        self,
        nodes: impl IntoIterator<Item = impl ErInput<Input>>,
    ) -> ErTree<Self::Error>
    where
        Self: Sized,
    {
        self.into_er_tree().er_add(nodes)
    }

    /// Erase the top, drops its traces. Use `into_er_tree().into_er_part()` to keep them.
    fn into_er_node(self) -> ErNode
    where
        Self: Sized,
        Self::Error: Error + Into<BoxError> + 'static,
    {
        self.into_er_tree().into_er_node()
    }

    /// Build your value from the owned report, keeping its chosen layout.
    /// Return fields, or the report itself to fill `into_*` fields automatically.
    /// Extra fields go beside the report: `|report| (report, (request_id, path))`.
    fn er_into<A, Mode>(self, fields: impl ErValFields<ErReport<Self::Error>, A, Mode>) -> A
    where
        Self: Sized,
    {
        fields.er_val_fields(self.into_er_report())
    }

    /// Just the outer error, moves the tree and keeps an existing layout.
    fn into_er_top(self) -> ErTop<Self::Error>
    where
        Self: Sized,
    {
        let report = self.into_er_report();
        ErTop {
            tree: report.tree,
            layout: report.layout,
        }
    }

    /// The whole report, moves the tree and keeps an existing layout.
    fn into_er_report(self) -> ErReport<Self::Error>
    where
        Self: Sized,
    {
        self.into_er_tree().into_er_report()
    }
}

/// Add stack traces to errors.
#[cfg(feature = "stack_traces")]
pub trait ErTraceExt: Sized {
    /// Capture the current stack for this layer. Leaves Ok alone.
    #[track_caller]
    fn er_trace(self) -> Self;
}

#[cfg(feature = "macros")]
#[doc(hidden)]
pub trait ErBail<T, Mode> {
    #[cfg_attr(feature = "src_locations", track_caller)]
    fn er_bail(self) -> T;
}

#[cfg(feature = "macros")]
#[doc(hidden)]
pub trait ErAllItem<Mode> {
    type Ok;

    #[cfg_attr(feature = "src_locations", track_caller)]
    fn er_all_item(self) -> Result<Self::Ok, ErPart>;
}
