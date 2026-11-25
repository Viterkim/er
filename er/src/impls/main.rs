use crate::{
    ErFields, ErFromFields, ErMainError, ErMainFailure, ErReportError, ErTree, IntoErTree,
};
use core::{error::Error, fmt};

impl<E> From<ErTree<E>> for ErMainFailure<E> {
    fn from(tree: ErTree<E>) -> Self {
        Self { tree }
    }
}
impl<E: Error + 'static> From<E> for ErMainFailure<E> {
    #[cfg_attr(feature = "src_locations", track_caller)]
    fn from(error: E) -> Self {
        Self {
            tree: ErTree::from(error),
        }
    }
}
impl<E: ErReportError<()>> fmt::Debug for ErMainFailure<E> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.tree.internal_report(), formatter)
    }
}
impl<E> IntoErTree for ErMainFailure<E> {
    type Error = E;

    fn into_er_tree(self) -> ErTree<E> {
        self.tree
    }
}

impl fmt::Display for ErMainError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("MainError")
    }
}
impl fmt::Debug for ErMainError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, formatter)
    }
}
impl Error for ErMainError {}
impl ErFromFields<(), ErFields> for ErMainError {
    fn er_from_fields((): ()) -> Self {
        Self
    }
}
