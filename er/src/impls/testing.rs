use crate::{ErTestError, ErTestFailure, ErTree, IntoErPart};
use core::{error::Error, fmt};

impl<E: IntoErPart> From<E> for ErTestFailure {
    #[cfg_attr(feature = "src_locations", track_caller)]
    fn from(error: E) -> Self {
        Self {
            tree: ErTree::new(ErTestError, [error]),
        }
    }
}
impl fmt::Display for ErTestFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.tree.er_report(), formatter)
    }
}
impl fmt::Debug for ErTestFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, formatter)
    }
}

impl fmt::Display for ErTestError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("TestError")
    }
}
impl fmt::Debug for ErTestError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, formatter)
    }
}
impl Error for ErTestError {}
impl From<()> for ErTestError {
    fn from(_: ()) -> Self {
        Self
    }
}
