#[cfg(feature = "macros")]
use crate::ErBoxedInput;
#[cfg(all(feature = "macros", any(er_unsync, target_has_atomic = "ptr")))]
use crate::impls::shared_tree::ErSharedNode;
use crate::internal::caller_location;
use crate::lines;
#[cfg(feature = "macros")]
use crate::{
    ErBail, ErBailFields, ErBailNodes, ErBailRoot, ErBailTestTop, ErBailTree, ErReady, ErWithReport,
};
use crate::{
    ErErrorContextExt, ErFields, ErFromFields, ErInput, ErLineError, ErNode, ErSavedList, ErTest,
    ErTestError, ErTestExt, ErTestFailure, ErTestInput, ErTestOptionNone, ErTestUnexpectedOk,
    ErTestUnit, ErTestWrongErr, ErTree, IntoErNode, IntoErTree,
};
#[cfg(feature = "macros")]
use core::marker::PhantomData;
use core::{error::Error, fmt, mem, ops::Deref};

#[cfg(feature = "macros")]
impl<P: ErInput<Mode>, Mode> ErBailTestTop<(ErReady, Mode)> for P {
    #[track_caller]
    fn into_test_node(self, _: u8) -> ErNode {
        self.into_er_input()
    }
}
#[cfg(feature = "macros")]
impl ErBailTestTop<ErTestUnit> for () {
    #[track_caller]
    fn into_test_node(self, _: u8) -> ErNode {
        ErTree::from(ErTestError).into_er_node()
    }
}
#[cfg(feature = "macros")]
impl<F: FnOnce(()) -> P, P: ErBailTestTop<Mode, Tag>, Mode, Tag>
    ErBailTestTop<(ErFields, Mode), Tag> for F
{
    #[track_caller]
    fn into_test_node(self, tag: Tag) -> ErNode {
        self(()).into_test_node(tag)
    }
}
#[cfg(feature = "macros")]
impl<P: ErBailTestTop<Mode, Tag>, Kind, Mode, Tag: Default>
    ErBailNodes<ErBailRoot<P, Kind>, ErTestFailure, (Mode, Tag)> for i32
{
    type Top = ErNode;

    #[track_caller]
    fn prepare(self, root: ErBailRoot<P, Kind>) -> (PhantomData<ErTestFailure>, ErTree<ErNode>) {
        let mut node = root.0.into_test_node(Tag::default());
        #[cfg(any(er_unsync, target_has_atomic = "ptr"))]
        if node.error.is::<ErSharedNode>() {
            // A shared branch is frozen. Put the other failures beside it under ErTestError.
            node = ErTree::new(ErTestError, [node]).into_er_node();
        }

        let tree = ErTree {
            nodes: mem::take(&mut node.nodes),
            ..ErTree::from_top(node)
        };
        (PhantomData, tree)
    }
}
#[cfg(feature = "macros")]
impl ErBailTree<ErTestFailure> for ErTree<ErNode> {
    #[track_caller]
    fn finish(self, _: PhantomData<ErTestFailure>) -> ErTestFailure {
        let mut node = self.top;
        node.nodes = self.nodes;
        ErTestFailure::from_node(node)
    }
}

#[cfg(feature = "macros")]
#[track_caller]
pub fn expect_error<T, E>(result: Result<T, E>) -> ErTest<E> {
    match result {
        Err(error) => Ok(error),
        Ok(_) => Err(ErTestFailure::from(ErTestUnexpectedOk)),
    }
}

impl ErTestFailure {
    #[cfg_attr(feature = "src_locations", track_caller)]
    pub fn new<Input>(error: impl ErInput<Input>) -> Self {
        Self::from_node(error.into_er_input())
    }

    #[inline(never)]
    #[cfg_attr(feature = "src_locations", track_caller)]
    fn from_node(mut node: ErNode) -> Self {
        let tree = if node.error.is::<ErTestError>() {
            ErTree {
                top: ErTestError,
                nodes: mem::take(&mut node.nodes),
                src_location: node.src_location.or(caller_location()),
                saved: mem::take(&mut node.saved),
            }
        } else {
            ErTree {
                top: ErTestError,
                nodes: alloc::vec![node],
                saved: ErSavedList::default(),
                src_location: caller_location(),
            }
        };

        Self { tree }
    }

    #[doc(hidden)]
    #[cfg_attr(feature = "src_locations", track_caller)]
    pub fn __er_expected<Input>(error: impl ErInput<Input>, expected: &'static str) -> Self {
        Self::new(ErTree::new(ErTestWrongErr { expected }, [error]))
    }

    /// Only formats once, no line endings.
    pub fn for_each_line(&self, emit: impl FnMut(&str)) -> fmt::Result {
        lines::for_each_line(self, emit)
    }

    /// Like `for_each_line()`, but the callback can fail.
    pub fn try_for_each_line<X>(
        &self,
        emit: impl FnMut(&str) -> Result<(), X>,
    ) -> Result<(), ErLineError<X>> {
        lines::try_for_each_line(self, emit)
    }
}
impl<E: IntoErNode> From<E> for ErTestFailure {
    #[cfg_attr(feature = "src_locations", track_caller)]
    fn from(error: E) -> Self {
        Self::new(error)
    }
}
impl fmt::Display for ErTestFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.tree.internal_report(), formatter)
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
impl ErInput for ErTestFailure {
    type Error = ErTestError;

    fn er_input_error(&self) -> &ErTestError {
        &self.tree.top
    }

    fn into_er_input(self) -> ErNode {
        self.tree.into_er_node()
    }
}
impl<Mode> ErErrorContextExt<Mode> for ErTestFailure {}

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
impl ErFromFields<(), ErFields> for ErTestError {
    fn er_from_fields((): ()) -> Self {
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

impl fmt::Display for ErTestUnexpectedOk {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("Expected an error, but it was Ok")
    }
}
impl fmt::Debug for ErTestUnexpectedOk {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, formatter)
    }
}
impl Error for ErTestUnexpectedOk {}

impl fmt::Display for ErTestWrongErr {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "Expected {}, but got the error below",
            self.expected
        )
    }
}
impl fmt::Debug for ErTestWrongErr {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, formatter)
    }
}
impl Error for ErTestWrongErr {}

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
impl<E: ErInput<Input>, Input> ErTestInput<Input> for E {
    #[cfg_attr(feature = "src_locations", track_caller)]
    fn into_er_test_failure(self) -> ErTestFailure {
        ErTestFailure::new(self)
    }
}
impl ErTestInput<ErTestUnit> for () {
    #[cfg_attr(feature = "src_locations", track_caller)]
    fn into_er_test_failure(self) -> ErTestFailure {
        ErTestFailure {
            tree: ErTree::from(ErTestError),
        }
    }
}

impl<T, E: ErTestInput<Input>, Input> ErTestExt<Input> for Result<T, E> {
    type Ok = T;

    #[cfg_attr(feature = "src_locations", track_caller)]
    fn er_test(self) -> ErTest<T> {
        match self {
            Ok(value) => Ok(value),
            Err(error) => Err(error.into_er_test_failure()),
        }
    }
}

#[cfg(feature = "macros")]
impl<F: FnOnce(()) -> E, E: ErInput<Input>, Input> ErBail<ErTestFailure, (ErFields, Input)> for F {
    #[cfg_attr(feature = "src_locations", track_caller)]
    fn er_bail(self) -> ErTestFailure {
        ErTestFailure::new(self(()))
    }
}
#[cfg(feature = "macros")]
impl<E: ErInput<Input>, Input> ErBailFields<ErTestFailure, (ErFields, Input)> for E {
    #[cfg_attr(feature = "src_locations", track_caller)]
    fn er_bail_fields(self) -> ErTestFailure {
        ErTestFailure::new(self)
    }
}

#[cfg(feature = "macros")]
impl<E: ErInput<ErWithReport<Input>>, Input> ErBail<ErTestFailure, ErWithReport<Input>> for E {
    #[cfg_attr(feature = "src_locations", track_caller)]
    fn er_bail(self) -> ErTestFailure {
        ErTestFailure::new(self)
    }
}
#[cfg(feature = "macros")]
impl<F: FnOnce() -> E, E: ErInput<ErWithReport<Input>>, Input>
    ErBail<ErTestFailure, (ErWithReport<Input>,)> for F
{
    #[cfg_attr(feature = "src_locations", track_caller)]
    fn er_bail(self) -> ErTestFailure {
        ErTestFailure::new(self())
    }
}

#[cfg(feature = "macros")]
impl<E: ErInput<ErBoxedInput>> ErBail<ErTestFailure, ErBoxedInput> for E {
    #[cfg_attr(feature = "src_locations", track_caller)]
    fn er_bail(self) -> ErTestFailure {
        ErTestFailure::new(self)
    }
}
#[cfg(feature = "macros")]
impl<F: FnOnce() -> E, E: ErInput<ErBoxedInput>> ErBail<ErTestFailure, (ErBoxedInput,)> for F {
    #[cfg_attr(feature = "src_locations", track_caller)]
    fn er_bail(self) -> ErTestFailure {
        ErTestFailure::new(self())
    }
}
