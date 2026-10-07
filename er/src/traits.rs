#[cfg(feature = "test")]
use crate::ErTest;
use crate::{BoxError, ErNode, ErPart, ErReport, ErResult, ErSnapshot, ErTop, ErTree};
use alloc::string::String;
use core::error::Error;

/// A finished error for `.er_with()` and its tree and Wrap forms.
pub struct ErBuilt<E>(pub E);

/// Use this error directly instead of building it from fields.
pub const fn er_built<E>(error: E) -> ErBuilt<E> {
    ErBuilt(error)
}

#[doc(hidden)]
pub struct ErReady;

/// Build the new top error from the fields returned by the closure.
pub struct ErFields;

#[cfg(er_unsync)]
#[doc(hidden)]
pub struct ErBoxedInput;

/// How `.er()` makes its new top error.
pub trait ErMake<A, Mode> {
    fn er_make(self) -> A;
}

impl<A, F: FnOnce() -> A> ErMake<A, ErReady> for F {
    fn er_make(self) -> A {
        self()
    }
}
impl<A: From<(P,)>, P, F: FnOnce(()) -> P> ErMake<A, ErFields> for F {
    fn er_make(self) -> A {
        A::from((self(()),))
    }
}

impl<A: From<()>> ErMake<A, ErFields> for () {
    fn er_make(self) -> A {
        A::from(())
    }
}

#[doc(hidden)]
pub trait ErWithFields<E: ?Sized, A, Mode>: for<'a> FnOnce(&'a E) -> Self::Input {
    type Input;

    fn er_with_fields(self, error: &E) -> A;
}
impl<E: ?Sized, A, Input, F> ErWithFields<E, A, ErFields> for F
where
    F: FnOnce(&E) -> Input,
    A: From<(Input,)>,
{
    type Input = Input;

    fn er_with_fields(self, error: &E) -> A {
        A::from((self(error),))
    }
}
impl<E: ?Sized, A, F> ErWithFields<E, A, ErReady> for F
where
    F: FnOnce(&E) -> ErBuilt<A>,
{
    type Input = ErBuilt<A>;

    fn er_with_fields(self, error: &E) -> A {
        self(error).0
    }
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
        self.er_build_tree(|error| fields.er_with_fields(error.er_input_error()))
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
        self.er_build_tree(|error| fields.er_with_fields(error))
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
    ///
    /// ! WARNING ! Don't use this to add context to an existing tree, you'll nuke it. Use `.er()` for that.
    ///
    /// ```rust,ignore
    /// // Err(85) calls DeviceErr::new(85)
    /// // Same as `|v| DeviceErr::new(v)`
    /// device_status().er_val(DeviceErr::new)?;
    /// ```
    #[cfg_attr(feature = "src_locations", track_caller)]
    fn er_val<A, F>(self, f: F) -> ErResult<Self::Ok, A>
    where
        A: Error + 'static,
        F: FnOnce(Self::Err) -> A;
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

    /// Convert to your error, the closure supplies its remaining fields from the failed tree.
    fn er_into<A>(self, fields: impl ErIntoFields<Self::Err, A>) -> Result<Self::Ok, A>
    where
        Self: Sized,
    {
        match self.er_tree() {
            Ok(value) => Ok(value),
            Err(tree) => Err(fields.er_into_tree(tree)),
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

/// Convert a tree into your own error, saving its diagnostics.
/// Generated by `#[derive(Er)]` for fields marked `into_top`,
/// `into_report_string` and `into_snapshot`.
/// Implement this yourself when another crate chooses the error type.
pub trait ErFromTree<E, Input = ()>: Sized {
    /// Save the diagnostics and turn the tree into your error.
    fn er_from_tree(tree: ErTree<E>, input: Input) -> Self;
}

#[doc(hidden)]
pub trait ErIntoFields<E, A>: for<'a> FnOnce(&'a ErTree<E>) -> Self::Input {
    type Input;

    fn er_into_tree(self, tree: ErTree<E>) -> A;
}
impl<E, A, Input, F> ErIntoFields<E, A> for F
where
    F: FnOnce(&ErTree<E>) -> Input,
    A: ErFromTree<E, Input>,
{
    type Input = Input;

    fn er_into_tree(self, tree: ErTree<E>) -> A {
        let values = self(&tree);
        A::er_from_tree(tree, values)
    }
}

/// Turn an Option or Result into ErTest. Existing test failures pass through.
#[cfg(feature = "test")]
pub trait ErTestExt<Input = ()> {
    type Ok;

    /// None reports "Option was None" with this call's location.
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

    /// Convert to your error, the closure supplies its remaining fields from the tree.
    fn er_into<A>(self, fields: impl ErIntoFields<Self::Error, A>) -> A
    where
        Self: Sized,
    {
        fields.er_into_tree(self.into_er_tree())
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
