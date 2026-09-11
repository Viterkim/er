use crate::{Er, ErNode, ErReport, ErTop, ErTree};
#[cfg(feature = "test")]
use crate::{TestEr, TestError};
use core::error::Error;

/// Start a tree from an error.
pub trait ErError: Error + Sized + 'static {
    /// Make a new Er error tree.
    ///
    /// `return Err(PortEr::new(85).er());`
    #[cfg_attr(feature = "src_locations", track_caller)]
    fn er(self) -> ErTree<Self> {
        ErTree::from(self)
    }
}

/// Add context to a Result, leave Ok alone.
pub trait ErResult {
    type Ok;
    type Err;

    /// Add your error on the top, move everything else below it.
    /// Only happens on failures.
    ///
    /// `read_port(input).er(ConfigEr::new)?;`
    #[cfg_attr(feature = "src_locations", track_caller)]
    fn er<A, F>(self, error: F) -> Er<Self::Ok, A>
    where
        A: Error + 'static,
        F: FnOnce() -> A,
        Self::Err: IntoErNode;

    /// Make an error from the failure value, starts fresh.
    ///
    /// `device_status().er_from(DeviceEr::new)?;`
    #[cfg_attr(feature = "src_locations", track_caller)]
    fn er_from<A, F>(self, error: F) -> Er<Self::Ok, A>
    where
        A: Error + 'static,
        F: FnOnce(Self::Err) -> A;

    #[cfg(feature = "test")]
    /// Add the test error, return a report.
    #[cfg_attr(feature = "src_locations", track_caller)]
    fn t_er(self) -> TestEr<Self::Ok>
    where
        Self: Sized,
        Self::Err: IntoErNode,
    {
        self.er(|| TestError).er_report()
    }
}

/// Turn None into your error.
pub trait ErOption {
    type Some;

    /// Make None into an error, like `.ok_or_else()`.
    ///
    /// `mode.er(ConfigEr::missing_mode)?;`
    #[cfg_attr(feature = "src_locations", track_caller)]
    fn er<A, F>(self, error: F) -> Er<Self::Some, A>
    where
        A: Error + 'static,
        F: FnOnce() -> A;

    #[cfg(feature = "test")]
    /// Add the test error, return a report.
    #[cfg_attr(feature = "src_locations", track_caller)]
    fn t_er(self) -> TestEr<Self::Some>
    where
        Self: Sized,
    {
        self.er(|| TestError).er_report()
    }
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
