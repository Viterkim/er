#[cfg(any(er_unsync, target_has_atomic = "ptr"))]
use crate::ErSharedTree;
use crate::aggregate::ErAggregateItems;
use crate::internal::ErWithFields;
#[cfg(feature = "macros")]
use crate::internal::{ErBail, ErBailFields, ErBailTestTop, ErFields, ErReady};
#[cfg(feature = "macros")]
use crate::{ErAllItem, ErTestFailure};
use crate::{
    ErBoxError, ErContextExt, ErErrorContextExt, ErErrorWithSaved, ErIteratorExt, ErMake, ErPart,
    ErPresentationExt, ErResult, ErResultExt, ErSaveInput, ErSaveRef, ErSavedList, ErSnapshot,
    ErTest, ErTestExt, ErTree, IntoErTree, IntoErTreeRef,
};
use alloc::string::String;
use core::error::Error;

/// Add context to an owned Wrap with `std_error`.
pub trait ErWrapExt: Error + IntoErTree + Sized {
    /// Take the tree out of a Wrap with `std_error` and add context.
    #[track_caller]
    fn er_wrap<A, Mode>(self, error: impl ErMake<A, Mode>) -> ErTree<A>
    where
        Self::Error: Error + Into<ErBoxError> + 'static,
        A: Error + 'static,
    {
        self.into_er_tree().er(error)
    }

    /// Take the tree out of a Wrap with `std_error` and build fields from its old top.
    #[track_caller]
    fn er_with_wrap<A, Mode>(self, fields: impl ErWithFields<Self::Error, A, Mode>) -> ErTree<A>
    where
        Self::Error: Error + Into<ErBoxError> + 'static,
        A: Error + 'static,
    {
        self.into_er_tree().er_with(fields)
    }

    /// Add the whole trees from failed `std_error` Wraps below the current top.
    #[track_caller]
    fn er_add_wrap<Kind>(
        self,
        items: impl ErAggregateItems<Kind, Item: ErWrappedItem>,
    ) -> ErTree<Self::Error> {
        self.into_er_tree().er_add_wrap(items)
    }

    /// Print the whole tree from a concrete Wrap, including a `Box<YourWrap>`.
    fn er_report_wrap(&self) -> String
    where
        for<'a> &'a Self: IntoErTreeRef<'a, Error = Self::Error>,
        Self::Error: Error + 'static,
    {
        self.into_er_tree_ref().er_report()
    }

    /// Copy the tree's messages and structure.
    fn er_snapshot_wrap(&self) -> ErSnapshot
    where
        for<'a> &'a Self: IntoErTreeRef<'a, Error = Self::Error>,
        Self::Error: Error + 'static,
    {
        self.into_er_tree_ref().er_snapshot()
    }

    /// Share the tree that the Wrap owns.
    #[cfg(any(er_unsync, target_has_atomic = "ptr"))]
    fn er_share_wrap(self) -> ErSharedTree<Self::Error> {
        self.into_er_tree().er_share()
    }

    /// Save a report on the inner tree of this Wrap.
    fn er_save_wrap(
        self,
        title: impl Into<String>,
        saved: impl ErSaveInput,
    ) -> ErTree<Self::Error> {
        self.into_er_tree().er_save(title, saved)
    }
}
impl<E: Error + IntoErTree> ErWrapExt for E {}

/// Keep the tree when a Result fails with a Wrap with `std_error`.
pub trait ErWrapResultExt: ErPresentationExt + Sized {
    /// Take the tree out of a Wrap with `std_error` and add context. Pass Ok through.
    ///
    /// Use this to keep the errors inside the Wrap searchable. A plain `.er()` or
    /// `.er_with()` boxes the Wrap, so a search can't see inside it.
    #[track_caller]
    fn er_wrap<A, Mode>(self, error: impl ErMake<A, Mode>) -> ErResult<Self::Ok, A>
    where
        Self::Err: Error + Into<ErBoxError> + 'static,
        A: Error + 'static,
    {
        self.er_tree().er(error)
    }

    /// Take the tree out of a Wrap with `std_error` and build fields from its old top.
    #[track_caller]
    fn er_with_wrap<A, Mode>(
        self,
        fields: impl ErWithFields<Self::Err, A, Mode>,
    ) -> ErResult<Self::Ok, A>
    where
        Self::Err: Error + Into<ErBoxError> + 'static,
        A: Error + 'static,
    {
        self.er_tree().er_with(fields)
    }

    /// Take the tree out of a Wrap with `std_error` and let the closure see the whole tree.
    #[track_caller]
    fn er_with_tree_wrap<A, Mode>(
        self,
        fields: impl ErWithFields<ErTree<Self::Err>, A, Mode>,
    ) -> ErResult<Self::Ok, A>
    where
        Self::Err: Error + Into<ErBoxError> + 'static,
        A: Error + 'static,
    {
        self.er_tree().er_with_tree(fields)
    }

    /// Keep the whole tree from a failed `std_error` Wrap as a test failure.
    #[track_caller]
    fn er_test_wrap(self) -> ErTest<Self::Ok>
    where
        Self::Err: Error + Into<ErBoxError> + 'static,
    {
        self.er_tree().er_test()
    }

    /// Save a report on the inner tree of a `std_error` Wrap.
    fn er_save_wrap<S>(
        self,
        title: impl Into<String>,
        saved: impl FnOnce(&ErTree<Self::Err>) -> &S,
    ) -> ErResult<Self::Ok, Self::Err>
    where
        S: ErSaveRef + ?Sized,
    {
        match self.er_tree() {
            Ok(value) => Ok(value),
            Err(tree) => Err(save_on_tree(tree, title, saved)),
        }
    }
}
impl<T, E: Error + IntoErTree> ErWrapResultExt for Result<T, E> {}

// Saving builds a String, built in the caller it made every successful call several times as expensive.
#[cold]
#[inline(never)]
fn save_on_tree<E, S: ErSaveRef + ?Sized>(
    mut tree: ErTree<E>,
    title: impl Into<String>,
    saved: impl FnOnce(&ErTree<E>) -> &S,
) -> ErTree<E> {
    let mut reports = ErSavedList::default();
    saved(&tree).er_save_ref(title.into(), &mut reports);
    tree.saved.append(&mut reports);

    tree
}

/// Collect results that fail with a Wrap with `std_error`.
pub trait ErWrapIteratorExt<Mode>: Sized {
    type Ok;

    /// Stop at the first failed `std_error` Wrap and keep its whole tree.
    #[track_caller]
    fn er_collect_wrap<C, A>(self, error: impl ErMake<A, Mode>) -> ErResult<C, A>
    where
        C: FromIterator<Self::Ok>,
        A: Error + 'static;

    /// Keep the trees from every failed `std_error` Wrap.
    #[track_caller]
    fn er_collect_all_wrap<C, A>(self, error: impl ErMake<A, Mode>) -> ErResult<C, A>
    where
        C: FromIterator<Self::Ok>,
        A: Error + 'static;
}
impl<I, T, E, Mode> ErWrapIteratorExt<Mode> for I
where
    I: Iterator<Item = Result<T, E>>,
    E: Error + IntoErTree,
    E::Error: Error + Into<ErBoxError> + 'static,
{
    type Ok = T;

    #[track_caller]
    fn er_collect_wrap<C, A>(self, error: impl ErMake<A, Mode>) -> ErResult<C, A>
    where
        C: FromIterator<Self::Ok>,
        A: Error + 'static,
    {
        self.map(ErPresentationExt::er_tree).er_collect(error)
    }

    #[track_caller]
    fn er_collect_all_wrap<C, A>(self, error: impl ErMake<A, Mode>) -> ErResult<C, A>
    where
        C: FromIterator<Self::Ok>,
        A: Error + 'static,
    {
        self.map(ErPresentationExt::er_tree).er_collect_all(error)
    }
}

impl<E> ErTree<E> {
    /// Add the whole trees from failed `std_error` Wraps below the current top.
    #[track_caller]
    pub fn er_add_wrap<Kind>(
        self,
        items: impl ErAggregateItems<Kind, Item: ErWrappedItem>,
    ) -> Self {
        self.er_add(items.er_items().map(ErWrappedItem::er_wrapped_item))
    }
}

/// An error that owns a tree but also implements `Error`, or a Result that may hold one.
#[doc(hidden)]
pub trait ErWrappedItem {
    type Ok;

    #[track_caller]
    fn er_wrapped_item(self) -> Result<Self::Ok, ErPart>;
}

// The macros pass an unsuffixed 0. Rust infers u8 for ordinary inputs and defaults to
// i32 when both impls fit a std_error Wrap. This lets later code infer the input type.
#[cfg(feature = "macros")]
pub trait ErAggregateItem<T, Mode> {
    type Ok;
    const ALWAYS_ERROR: bool;

    #[track_caller]
    fn item(self, input: T) -> Result<Self::Ok, ErPart>;
}

#[cfg(feature = "macros")]
// er_try! needs the Ok type before integer fallback, for example when the next line calls text.len().
pub struct ErMacroItem<T, Mode>(pub T, pub core::marker::PhantomData<Mode>);

#[cfg(feature = "macros")]
impl<T: ErAllItem<Mode>, Mode> ErMacroItem<T, Mode> {
    #[track_caller]
    pub fn item<Kind>(
        self,
        tag: impl ErAggregateItem<T, Kind, Ok = T::Ok>,
    ) -> Result<T::Ok, ErPart> {
        tag.item(self.0)
    }
}

#[cfg(feature = "macros")]
pub trait ErWrappedFallback {
    type Ok;

    #[track_caller]
    fn item(self, tag: i32) -> Result<Self::Ok, ErPart>;
}

#[cfg(feature = "macros")]
impl<T: ErWrappedItem> ErWrappedFallback for ErMacroItem<T, ()> {
    type Ok = T::Ok;

    #[track_caller]
    fn item(self, _: i32) -> Result<T::Ok, ErPart> {
        self.0.er_wrapped_item()
    }
}

#[cfg(feature = "macros")]
pub trait ErAggregateAdd<I, Mode> {
    #[track_caller]
    fn add<T: IntoErTree>(self, tree: T, items: I) -> ErTree<T::Error>;
}

#[cfg(feature = "macros")]
pub struct ErAssertValue<T>(pub T);

#[cfg(feature = "macros")]
pub trait ErAssertInput<R, Input> {
    type Error;

    #[track_caller]
    fn test_err(self, result: R) -> ErTest<ErAssertValue<Self::Error>>;
}

// Ordinary ErResults already know how to keep their trees. Only std_error Wraps also look like plain errors.
pub trait WrappedTree {
    type Error;

    fn into_wrapped_tree(self) -> ErTree<Self::Error>;
}
impl<E: Error + IntoErTree> WrappedTree for E {
    type Error = E::Error;

    fn into_wrapped_tree(self) -> ErTree<Self::Error> {
        self.into_er_tree()
    }
}
impl<E: WrappedTree> WrappedTree for ErErrorWithSaved<E> {
    type Error = E::Error;

    fn into_wrapped_tree(self) -> ErTree<Self::Error> {
        let Self { value, mut saved } = self;
        let mut tree = value.into_wrapped_tree();
        tree.saved.append(&mut saved);
        tree
    }
}

impl<E: WrappedTree> ErWrappedItem for E
where
    E::Error: Error + Into<ErBoxError> + 'static,
{
    type Ok = ();

    #[track_caller]
    fn er_wrapped_item(self) -> Result<(), ErPart> {
        Err(self.into_wrapped_tree().into_er_part())
    }
}
impl<T, E: WrappedTree> ErWrappedItem for Result<T, E>
where
    E::Error: Error + Into<ErBoxError> + 'static,
{
    type Ok = T;

    #[track_caller]
    fn er_wrapped_item(self) -> Result<T, ErPart> {
        match self {
            Ok(value) => Ok(value),
            Err(error) => Err(error.into_wrapped_tree().into_er_part()),
        }
    }
}
impl<T: ErWrappedItem> ErWrappedItem for Option<T> {
    type Ok = Option<T::Ok>;

    #[track_caller]
    fn er_wrapped_item(self) -> Result<Self::Ok, ErPart> {
        match self {
            Some(item) => item.er_wrapped_item().map(Some),
            None => Ok(None),
        }
    }
}

#[cfg(feature = "macros")]
#[doc(hidden)]
pub trait ErBailValue<P, T, Mode> {
    #[track_caller]
    fn bail(self, input: P) -> T;
}

#[cfg(feature = "macros")]
impl<P: WrappedTree> ErBailTestTop<(), i32> for P
where
    P::Error: Error + Into<ErBoxError> + 'static,
{
    #[track_caller]
    fn into_test_part(self, _: i32) -> ErPart {
        self.into_wrapped_tree().into_er_part()
    }
}

#[cfg(feature = "macros")]
impl<F: FnOnce() -> P, P: ErBailTestTop<Mode, i32>, Mode> ErBailTestTop<(ErReady, Mode), i32>
    for F
{
    #[track_caller]
    fn into_test_part(self, tag: i32) -> ErPart {
        self().into_test_part(tag)
    }
}

#[cfg(feature = "macros")]
impl<P: ErBailTestTop<Mode, i32>, Kind, Mode> ErBailValue<P, ErTestFailure, Mode> for (i32, Kind) {
    #[track_caller]
    fn bail(self, input: P) -> ErTestFailure {
        ErTestFailure::new(input.into_test_part(0))
    }
}

#[cfg(feature = "macros")]
impl<P: ErBail<T, Mode>, T, Mode> ErBailValue<P, T, Mode> for (u8, ErReady) {
    #[track_caller]
    fn bail(self, input: P) -> T {
        input.er_bail()
    }
}

#[cfg(feature = "macros")]
impl<P: ErBailFields<T, Mode>, T, Mode> ErBailValue<P, T, Mode> for (u8, ErFields) {
    #[track_caller]
    fn bail(self, input: P) -> T {
        input.er_bail_fields()
    }
}

#[cfg(feature = "macros")]
impl<T, E: WrappedTree> ErAssertInput<Result<T, E>, ()> for i32
where
    E::Error: Error + Into<ErBoxError> + 'static,
{
    type Error = ErTree<E::Error>;

    #[track_caller]
    fn test_err(self, result: Result<T, E>) -> ErTest<ErAssertValue<Self::Error>> {
        let result = match result {
            Ok(value) => Ok(value),
            Err(error) => Err(error.into_wrapped_tree()),
        };

        crate::impls::testing::expect_error(result).map(ErAssertValue)
    }
}

#[cfg(feature = "macros")]
impl<R: ErResultExt<Input>, Input> ErAssertInput<R, Input> for u8 {
    type Error = R::Err;

    #[track_caller]
    fn test_err(self, result: R) -> ErTest<ErAssertValue<Self::Error>> {
        crate::impls::testing::expect_error(result.er_result_input()).map(ErAssertValue)
    }
}

#[cfg(feature = "macros")]
impl<T: ErWrappedItem> ErAggregateItem<T, ()> for i32 {
    type Ok = T::Ok;
    const ALWAYS_ERROR: bool = false;

    #[track_caller]
    fn item(self, input: T) -> Result<Self::Ok, ErPart> {
        input.er_wrapped_item()
    }
}

#[cfg(feature = "macros")]
impl<T: ErAllItem<Mode>, Mode> ErAggregateItem<T, Mode> for u8 {
    type Ok = T::Ok;
    const ALWAYS_ERROR: bool = T::ALWAYS_ERROR;

    #[track_caller]
    fn item(self, input: T) -> Result<Self::Ok, ErPart> {
        input.er_all_item()
    }
}

#[cfg(feature = "macros")]
impl<I: Iterator> ErAggregateAdd<I, ()> for i32
where
    I::Item: ErWrappedItem,
{
    #[track_caller]
    fn add<T: IntoErTree>(self, tree: T, items: I) -> ErTree<T::Error> {
        tree.into_er_tree().er_add_wrap(items)
    }
}

#[cfg(feature = "macros")]
impl<I: Iterator, Mode> ErAggregateAdd<I, Mode> for u8
where
    I::Item: ErAllItem<Mode>,
{
    #[track_caller]
    fn add<T: IntoErTree>(self, tree: T, items: I) -> ErTree<T::Error> {
        tree.into_er_tree().er_add(items)
    }
}
