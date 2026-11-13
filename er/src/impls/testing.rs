#[cfg(all(feature = "macros", er_unsync))]
use crate::ErBoxedInput;
#[cfg(all(feature = "macros", any(er_unsync, target_has_atomic = "ptr")))]
use crate::impls::shared_tree::ErSharedNode;
use crate::lines;
#[cfg(feature = "macros")]
use crate::{
    ErBail, ErBailFields, ErBailNodes, ErBailRoot, ErBailTestTop, ErBailTree, ErNode, ErReady,
    ErWithReport,
};
use crate::{
    ErErrorContextExt, ErFailed, ErFailure, ErFields, ErFromFields, ErInput, ErLineError, ErPart,
    ErTest, ErTestExt, ErTestInput, ErTestOptionNone, ErTestUnexpectedOk, ErTestUnit,
    ErTestWrongErr, ErTree, IntoErPart, IntoErTree,
};
#[cfg(feature = "macros")]
use core::marker::PhantomData;
#[cfg(feature = "src_locations")]
use core::panic::Location;
use core::{error::Error, fmt, mem, ops::Deref};

#[cfg(feature = "macros")]
impl<P: ErInput<Mode>, Mode> ErBailTestTop<(ErReady, Mode)> for P {
    #[track_caller]
    fn into_test_part(self, _: u8) -> ErPart {
        self.into_er_input()
    }
}
#[cfg(feature = "macros")]
impl ErBailTestTop<ErTestUnit> for () {
    #[track_caller]
    fn into_test_part(self, _: u8) -> ErPart {
        ErFailed.into_er_part()
    }
}
#[cfg(feature = "macros")]
impl<F: FnOnce(()) -> P, P: ErBailTestTop<Mode, Tag>, Mode, Tag>
    ErBailTestTop<(ErFields, Mode), Tag> for F
{
    #[track_caller]
    fn into_test_part(self, tag: Tag) -> ErPart {
        self(()).into_test_part(tag)
    }
}
#[cfg(feature = "macros")]
impl<P: ErBailTestTop<Mode, Tag>, Kind, Mode, Tag: Default>
    ErBailNodes<ErBailRoot<P, Kind>, ErFailure, (Mode, Tag)> for i32
{
    type Top = ErNode;

    #[track_caller]
    fn prepare(self, root: ErBailRoot<P, Kind>) -> (PhantomData<ErFailure>, ErTree<ErNode>) {
        let part = root.0.into_test_part(Tag::default());
        #[cfg(any(er_unsync, target_has_atomic = "ptr"))]
        let part = if part.node.error.is::<ErSharedNode>() {
            // A shared branch is frozen. Put the other failures beside it under ErFailed.
            ErTree::new(ErFailed, [part]).into_er_part()
        } else {
            part
        };

        let ErPart { mut node, saved } = part;
        let tree = ErTree {
            nodes: mem::take(&mut node.nodes),
            saved,
            #[cfg(feature = "src_locations")]
            src_location: node.src_location,
            top: node,
        };
        (PhantomData, tree)
    }
}
#[cfg(feature = "macros")]
impl ErBailTree<ErFailure> for ErTree<ErNode> {
    #[track_caller]
    fn finish(self, _: PhantomData<ErFailure>) -> ErFailure {
        let mut node = self.top;
        node.nodes = self.nodes;
        ErFailure::from_part(ErPart {
            node,
            saved: self.saved,
        })
    }
}

#[cfg(feature = "macros")]
#[track_caller]
pub fn expect_error<T, E>(result: Result<T, E>) -> ErTest<E> {
    match result {
        Err(error) => Ok(error),
        Ok(_) => Err(ErFailure::from(ErTestUnexpectedOk)),
    }
}

impl ErFailure {
    #[cfg_attr(feature = "src_locations", track_caller)]
    pub fn new<Input>(error: impl ErInput<Input>) -> Self {
        Self::from_part(error.into_er_input())
    }

    #[inline(never)]
    #[cfg_attr(feature = "src_locations", track_caller)]
    fn from_part(part: ErPart) -> Self {
        let tree = if part.node.error.is::<ErFailed>() {
            let ErPart { mut node, saved } = part;
            ErTree {
                top: ErFailed,
                nodes: mem::take(&mut node.nodes),
                #[cfg(feature = "src_locations")]
                src_location: node.src_location,
                saved,
            }
        } else {
            ErTree {
                top: ErFailed,
                nodes: alloc::vec![part.node],
                saved: part.saved,
                #[cfg(feature = "src_locations")]
                src_location: Location::caller(),
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
impl<E: IntoErPart> From<E> for ErFailure {
    #[cfg_attr(feature = "src_locations", track_caller)]
    fn from(error: E) -> Self {
        Self::new(error)
    }
}
impl fmt::Display for ErFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.tree.internal_report(), formatter)
    }
}
impl fmt::Debug for ErFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, formatter)
    }
}

impl Deref for ErFailure {
    type Target = ErTree<ErFailed>;

    fn deref(&self) -> &Self::Target {
        &self.tree
    }
}
impl IntoErTree for ErFailure {
    type Error = ErFailed;

    fn into_er_tree(self) -> ErTree<ErFailed> {
        self.tree
    }
}
impl ErInput for ErFailure {
    type Error = ErFailed;

    fn er_input_error(&self) -> &ErFailed {
        &self.tree.top
    }

    fn into_er_input(self) -> ErPart {
        self.tree.into_er_part()
    }
}
impl<Mode> ErErrorContextExt<Mode> for ErFailure {}

impl fmt::Display for ErFailed {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("Failed")
    }
}
impl fmt::Debug for ErFailed {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, formatter)
    }
}
impl Error for ErFailed {}
impl ErFromFields<(), ErFields> for ErFailed {
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
            None => Err(ErFailure::from(ErTestOptionNone)),
        }
    }
}
impl<E: ErInput<Input>, Input> ErTestInput<Input> for E {
    #[cfg_attr(feature = "src_locations", track_caller)]
    fn into_er_test_failure(self) -> ErFailure {
        ErFailure::new(self)
    }
}
impl ErTestInput<ErTestUnit> for () {
    #[cfg_attr(feature = "src_locations", track_caller)]
    fn into_er_test_failure(self) -> ErFailure {
        ErFailure::new(ErFailed)
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
impl<F: FnOnce(()) -> E, E: ErInput<Input>, Input> ErBail<ErFailure, (ErFields, Input)> for F {
    #[cfg_attr(feature = "src_locations", track_caller)]
    fn er_bail(self) -> ErFailure {
        ErFailure::new(self(()))
    }
}
#[cfg(feature = "macros")]
impl<E: ErInput<Input>, Input> ErBailFields<ErFailure, (ErFields, Input)> for E {
    #[cfg_attr(feature = "src_locations", track_caller)]
    fn er_bail_fields(self) -> ErFailure {
        ErFailure::new(self)
    }
}

#[cfg(feature = "macros")]
impl<E: ErInput<ErWithReport<Input>>, Input> ErBail<ErFailure, ErWithReport<Input>> for E {
    #[cfg_attr(feature = "src_locations", track_caller)]
    fn er_bail(self) -> ErFailure {
        ErFailure::new(self)
    }
}
#[cfg(feature = "macros")]
impl<F: FnOnce() -> E, E: ErInput<ErWithReport<Input>>, Input>
    ErBail<ErFailure, (ErWithReport<Input>,)> for F
{
    #[cfg_attr(feature = "src_locations", track_caller)]
    fn er_bail(self) -> ErFailure {
        ErFailure::new(self())
    }
}

#[cfg(all(feature = "macros", er_unsync))]
impl<E: ErInput<ErBoxedInput>> ErBail<ErFailure, ErBoxedInput> for E {
    #[cfg_attr(feature = "src_locations", track_caller)]
    fn er_bail(self) -> ErFailure {
        ErFailure::new(self)
    }
}
#[cfg(all(feature = "macros", er_unsync))]
impl<F: FnOnce() -> E, E: ErInput<ErBoxedInput>> ErBail<ErFailure, (ErBoxedInput,)> for F {
    #[cfg_attr(feature = "src_locations", track_caller)]
    fn er_bail(self) -> ErFailure {
        ErFailure::new(self())
    }
}
