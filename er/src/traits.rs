use crate::{ErPart, ErReport, ErResult, ErTop, ErTree};
use core::error::Error;

/// Use the error returned by the closure as the new top error.
pub struct ErBuilt;

/// Build the new top error from the fields returned by the closure.
pub struct ErFields;

/// How `.er()` makes its new top error.
pub trait ErMake<A, Mode> {
    fn er_make(self) -> A;
}

impl<A, F: FnOnce() -> A> ErMake<A, ErBuilt> for F {
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

/// Start a tree from an error.
pub trait ErErrorExt: Error + Sized + 'static {
    /// Make a new Er error tree.
    ///
    /// `let tree = PortErr::new(85).er();`
    ///
    /// For adding context to a Result, see [`ErContextExt::er`].
    #[cfg_attr(feature = "src_locations", track_caller)]
    fn er(self) -> ErTree<Self> {
        ErTree::from(self)
    }

    /// Add your error on the top, move everything else below it.
    /// |e| is the old error.
    /// Use this when the new error needs something from the old one.
    ///
    /// `er_bail!(device.er_with(|e| AnalyzeErr::new(e.code)));`
    #[cfg_attr(feature = "src_locations", track_caller)]
    fn er_with<A>(self, error: impl FnOnce(&Self) -> A) -> ErTree<A>
    where
        Self: Send + Sync,
        A: Error + 'static,
    {
        let top = error(&self);
        ErTree::new(top, [self])
    }
}

/// Add context to a Result or turn None into an error.
pub trait ErContextExt<Mode> {
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
    #[cfg_attr(feature = "src_locations", track_caller)]
    fn er<A>(self, error: impl ErMake<A, Mode>) -> ErTree<A>
    where
        A: Error + 'static;
}

/// Other ways to work with a Result's error.
pub trait ErResultExt {
    type Ok;
    type Err;

    /// Add your error on the top, move everything else below it.
    /// Only happens on failures.
    /// If the Err is already an Er tree, |t| is the tree. The error is `t.top`.
    /// Use this when the new error needs something from the old one.
    /// Otherwise use `.er()`.
    ///
    /// `result.er_with(|t| AnalyzeErr::new(t.top.code))?;`
    #[cfg_attr(feature = "src_locations", track_caller)]
    fn er_with<A>(self, error: impl FnOnce(&Self::Err) -> A) -> ErResult<Self::Ok, A>
    where
        A: Error + 'static,
        Self::Err: IntoErPart;

    /// For values that don't implement `Error`, like `Err(85)`.
    ///
    /// ! WARNING ! Don't use this to add context to an existing tree, you'll nuke it. Use `.er()` for that.
    ///
    /// ```rust,ignore
    /// // Err(85) calls DeviceErr::new(85)
    /// // Same as `|v| DeviceErr::new(v)`
    /// device_status().er_val(DeviceErr::new)?;
    /// ```
    #[cfg_attr(feature = "src_locations", track_caller)]
    fn er_val<A, F>(self, error: F) -> ErResult<Self::Ok, A>
    where
        A: Error + 'static,
        F: FnOnce(Self::Err) -> A;
}

/// Collect successful values and add context to failures.
pub trait ErIteratorExt<Mode>: Sized {
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

    /// Take the tree out of a Wrap with `std_error` and add context. Leaves Ok alone.
    ///
    /// !WARNING! Normal `.er()` boxes a Wrap with `std_error`, so you can't find the errors inside it.
    #[cfg_attr(feature = "src_locations", track_caller)]
    fn er_wrap<A, Mode>(self, error: impl ErMake<A, Mode>) -> ErResult<Self::Ok, A>
    where
        Self: Sized,
        Self::Err: Error + Send + Sync + 'static,
        A: Error + 'static,
    {
        self.er_tree().er(error)
    }
}

/// Give a presentation standard Error support. On a Result, leaves Ok alone.
pub trait ErOpaqueErrorExt {
    type Output;

    /// The presentation stays in `.0`, but error searches can't see inside it.
    fn opaque_err(self) -> Self::Output;
}

/// Turns an error or tree into a node and its metadata.
pub trait IntoErPart {
    #[cfg_attr(feature = "src_locations", track_caller)]
    fn into_er_part(self) -> ErPart;
}

/// Get the tree out of a tree, Wrap or owned presentation. Leaves the old layout behind.
pub trait IntoErTree {
    type Error;

    fn into_er_tree(self) -> ErTree<Self::Error>;

    /// Just the outer error, moves the tree.
    fn into_er_top(self) -> ErTop<Self::Error>
    where
        Self: Sized,
    {
        self.into_er_tree().into_er_top()
    }

    /// The whole report, moves the tree.
    fn into_er_report(self) -> ErReport<Self::Error>
    where
        Self: Sized,
    {
        self.into_er_tree().into_er_report()
    }
}

#[cfg(feature = "stack_traces")]
pub trait ErTraceExt: Sized {
    /// Capture the current stack for this layer. Leaves Ok alone.
    #[track_caller]
    fn er_trace(self) -> Self;
}

#[cfg(feature = "macros")]
#[doc(hidden)]
pub trait ErAllItem<Mode> {
    #[cfg_attr(feature = "src_locations", track_caller)]
    fn er_all_item(self) -> Result<(), ErPart>;
}
