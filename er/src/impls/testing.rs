#[cfg(feature = "macros")]
use crate::{ErBail, ErFields};
use crate::{
    ErInput, ErMake, ErPart, ErTest, ErTestError, ErTestExt, ErTestFailure, ErTestOptionNone,
    ErTree, ErTreeContextExt, IntoErPart, IntoErTree,
};
use core::{error::Error, fmt, mem, ops::Deref};

impl<E: IntoErPart> From<E> for ErTestFailure {
    #[cfg_attr(feature = "src_locations", track_caller)]
    fn from(error: E) -> Self {
        let mut part = error.into_er_part();
        let tree = if part.node.error.is::<ErTestError>() {
            ErTree {
                top: ErTestError,
                nodes: mem::take(&mut part.node.nodes),
                #[cfg(feature = "src_locations")]
                src_location: part.node.src_location,
                #[cfg(feature = "stack_traces")]
                stack_traces: part.stack_traces,
            }
        } else {
            ErTree::new(ErTestError, [part])
        };

        Self { tree }
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
impl Deref for ErTestFailure {
    type Target = ErTree<ErTestError>;

    fn deref(&self) -> &Self::Target {
        &self.tree
    }
}
impl IntoErTree for ErTestFailure {
    type Error = ErTestError;

    fn into_er_tree(self) -> ErTree<ErTestError> {
        self.tree
    }
}
impl AsMut<ErTree<ErTestError>> for ErTestFailure {
    fn as_mut(&mut self) -> &mut ErTree<ErTestError> {
        &mut self.tree
    }
}
impl ErInput for ErTestFailure {
    type Error = ErTestError;

    fn er_input_error(&self) -> &ErTestError {
        &self.tree.top
    }

    fn into_er_input(self) -> ErPart {
        self.tree.into_er_part()
    }
}
impl<Mode> ErTreeContextExt<Mode> for ErTestFailure {
    #[cfg_attr(feature = "src_locations", track_caller)]
    fn er<A>(self, error: impl ErMake<A, Mode>) -> ErTree<A>
    where
        A: Error + 'static,
    {
        self.tree.er(error)
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

impl fmt::Display for ErTestOptionNone {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("Option was None")
    }
}
impl fmt::Debug for ErTestOptionNone {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, formatter)
    }
}
impl Error for ErTestOptionNone {}

impl<T> ErTestExt for Option<T> {
    type Ok = T;

    #[cfg_attr(feature = "src_locations", track_caller)]
    fn er_test(self) -> ErTest<T> {
        match self {
            Some(value) => Ok(value),
            None => Err(ErTestFailure::from(ErTestOptionNone)),
        }
    }
}
impl<T, E: Into<ErTestFailure>> ErTestExt for Result<T, E> {
    type Ok = T;

    #[cfg_attr(feature = "src_locations", track_caller)]
    fn er_test(self) -> ErTest<T> {
        match self {
            Ok(value) => Ok(value),
            Err(error) => Err(error.into()),
        }
    }
}

#[cfg(feature = "macros")]
impl<F: FnOnce(()) -> E, E: Into<ErTestFailure>> ErBail<ErTestFailure, ErFields> for F {
    #[cfg_attr(feature = "src_locations", track_caller)]
    fn er_bail(self) -> ErTestFailure {
        self(()).into()
    }
}
