use crate::{Er, ErNode, ErReport, ErTop, ErTree};
use core::error::Error;

/// Use the error returned by the closure as the new top error.
pub struct ErBuilt;

/// Build the new top error from the fields returned by the closure.
pub struct ErFields;

/// Turn callback output into the new top error.
pub trait ErPayload<A, Mode> {
    fn er_payload(self) -> A;
}

impl<A> ErPayload<A, ErBuilt> for A {
    fn er_payload(self) -> A {
        self
    }
}

impl<A: From<(P,)>, P> ErPayload<A, ErFields> for P {
    fn er_payload(self) -> A {
        A::from((self,))
    }
}

/// How `.er()` makes its new top error.
pub trait ErMake<A, Mode> {
    fn er_make(self) -> A;
}

impl<A, F: FnOnce() -> A> ErMake<A, ErBuilt> for F {
    fn er_make(self) -> A {
        self()
    }
}

impl<A: From<(P,)>, P, F: FnOnce() -> P> ErMake<A, ErFields> for F {
    fn er_make(self) -> A {
        ErPayload::<A, ErFields>::er_payload(self())
    }
}

impl<A: From<()>> ErMake<A, ErFields> for () {
    fn er_make(self) -> A {
        A::from(())
    }
}

/// Start a tree from an error.
pub trait ErError: Error + Sized + 'static {
    /// Make a new Er error tree.
    ///
    /// `return Err(PortErr::new(85).er());`
    ///
    /// For adding context to a Result, see [`ErContext::er`].
    #[cfg_attr(feature = "src_locations", track_caller)]
    fn er(self) -> ErTree<Self> {
        ErTree::from(self)
    }

    /// Add your error on the top, move everything else below it.
    /// |e| is the old error.
    /// Use this when the new error needs something from the old one.
    ///
    /// `return Err(device.er_with(|e| e.code));`
    #[cfg_attr(feature = "src_locations", track_caller)]
    fn er_with<A, P, Mode>(self, error: impl FnOnce(&Self) -> P) -> ErTree<A>
    where
        Self: Send + Sync,
        A: Error + 'static,
        P: ErPayload<A, Mode>,
    {
        let parent = error(&self).er_payload();
        ErTree::new(parent, [self])
    }
}

/// Add context to a Result or turn None into an error.
pub trait ErContext<Mode> {
    type Ok;

    /// Add your error on the top, move everything else below it.
    /// Only happens on Err or None.
    ///
    /// ```rust,ignore
    /// result.er(())?; // Empty struct
    /// result.er(|| path)?; // One field
    /// result.er(|| (machine, token))?; // More fields
    /// result.er(|| EnumErr::variant_name(arg1))?; // Enum variant
    /// ```
    #[cfg_attr(feature = "src_locations", track_caller)]
    fn er<A>(self, error: impl ErMake<A, Mode>) -> Er<Self::Ok, A>
    where
        A: Error + 'static;
}

/// Add a new top error above an existing tree.
pub trait ErTreeContext<Mode> {
    #[cfg_attr(feature = "src_locations", track_caller)]
    fn er<A>(self, error: impl ErMake<A, Mode>) -> ErTree<A>
    where
        A: Error + 'static;
}

/// Other ways to work with a Result's error.
pub trait ErResult {
    type Ok;
    type Err;

    /// Add your error on the top, move everything else below it.
    /// Only happens on failures.
    /// If the Err is already an Er tree, |t| is the tree. The error is `t.top`.
    /// Use this when the new error needs something from the old one.
    /// Otherwise use `.er()`.
    ///
    /// `result.er_with(|t| t.top.code)?;`
    #[cfg_attr(feature = "src_locations", track_caller)]
    fn er_with<A, P, Mode>(self, error: impl FnOnce(&Self::Err) -> P) -> Er<Self::Ok, A>
    where
        A: Error + 'static,
        P: ErPayload<A, Mode>,
        Self::Err: IntoErNode;

    /// For values that don't implement `Error`, like `Err(85)`.
    ///
    /// !WARNING! Don't use this to add context to an existing tree, you'll nuke it.
    /// Use `.er()` for that.
    ///
    /// ```rust,ignore
    /// // Err(85) calls DeviceErr::new(85)
    /// // Same as `|v| DeviceErr::new(v)`
    /// device_status().er_val(DeviceErr::new)?;
    /// ```
    #[cfg_attr(feature = "src_locations", track_caller)]
    fn er_val<A, F>(self, error: F) -> Er<Self::Ok, A>
    where
        A: Error + 'static,
        F: FnOnce(Self::Err) -> A;
}

/// Get the tree or pick the output for a Result with a tree, Wrap or owned presentation.
pub trait ErPresentation {
    type Ok;
    type Err;

    /// Get the normal Er result back.
    fn er_tree(self) -> Er<Self::Ok, Self::Err>;

    /// Just the outer error, leaves Ok alone.
    ///
    /// `read_port("85").er_top().unwrap();`
    fn er_top(self) -> Result<Self::Ok, ErTop<Self::Err>>;

    /// The whole report, leaves Ok alone.
    ///
    /// `read_port("85").er_report().unwrap();`
    fn er_report(self) -> Result<Self::Ok, ErReport<Self::Err>>;

    /// Take the tree out of a Wrap with `std_error` and add context. Leaves Ok alone.
    ///
    /// !WARNING! Normal `.er()` boxes a Wrap with `std_error`, so you can't find its children.
    #[cfg_attr(feature = "src_locations", track_caller)]
    fn er_wrap<A, F>(self, error: F) -> Er<Self::Ok, A>
    where
        Self: Sized,
        Self::Err: Error + Send + Sync + 'static,
        A: Error + 'static,
        F: FnOnce() -> A,
    {
        self.er_tree().er(error)
    }
}

/// Give a presentation standard Error support. On a Result, leaves Ok alone.
pub trait ErOpaqueError {
    type Output;

    /// The presentation stays in `.0`, but error searches can't see inside it.
    fn opaque_err(self) -> Self::Output;
}

/// Turns an error or tree into a node.
pub trait IntoErNode {
    #[cfg_attr(feature = "src_locations", track_caller)]
    fn into_er_node(self) -> ErNode;
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
