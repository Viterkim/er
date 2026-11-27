#[cfg(er_unsync)]
use crate::ErBoxedInput;
use crate::impls::tree::{build_tree, with_source};
use crate::internal::{ErWithFields, er_make_cold};
use crate::{ErAllItem, ErAllOption, ErAllResult, ErNode};
#[cfg(feature = "macros")]
use crate::{
    ErBail, ErBailFields, ErBailTarget, ErBuildFrom, ErErrorWithSaved, ErFieldInput, ErFieldsFor,
    ErForm, ErFromFields, ErReady, ErValueForm, ErWithSaves,
};
use crate::{
    ErBorrowedPresentationExt, ErBoxError, ErContextExt, ErErrorContextExt, ErErrorExt, ErInput,
    ErMake, ErPollExt, ErPresentationExt, ErReportError, ErResult, ErResultExt, ErSnapshot,
    ErSnapshotPresentationExt, ErTree, ErValueExt, IntoErSnapshot, IntoErTree, IntoErTreeRef,
};
#[cfg(er_unsync)]
use alloc::boxed::Box;
use alloc::string::String;
#[cfg(feature = "macros")]
use alloc::vec::Vec;
use core::error::Error;
use core::task::Poll;

impl<T: Into<ErBoxError>, Mode> ErErrorContextExt<Mode> for T {}
impl<T: Into<ErBoxError>> ErErrorExt for T {}

#[cfg(er_unsync)]
impl<Mode> ErErrorContextExt<Mode, ErBoxedInput> for Box<dyn Error + Send + Sync> {}
#[cfg(er_unsync)]
impl<Mode> ErErrorContextExt<Mode, ErBoxedInput> for Box<dyn Error + Send> {}
#[cfg(er_unsync)]
impl<Mode> ErErrorContextExt<Mode, ErBoxedInput> for Box<dyn Error + Sync> {}
#[cfg(er_unsync)]
impl ErErrorExt<ErBoxedInput> for Box<dyn Error + Send + Sync> {}
#[cfg(er_unsync)]
impl ErErrorExt<ErBoxedInput> for Box<dyn Error + Send> {}
#[cfg(er_unsync)]
impl ErErrorExt<ErBoxedInput> for Box<dyn Error + Sync> {}

impl<T, E: ErInput<Input>, Mode, Input> ErContextExt<Mode, Input> for Result<T, E> {
    type Ok = T;

    #[track_caller]
    fn er<A>(self, error: impl ErMake<A, Mode>) -> ErResult<T, A>
    where
        A: Error + 'static,
    {
        match self {
            Ok(value) => Ok(value),
            // Plain data is cheap to build right here. A String conversion or a closure isn't, and building it in
            // every caller made them too big to inline into each other: successful calls ran 2.6 times slower.
            // `ErMake::COLD` tells them apart at compile time.
            Err(source) if !er_make_cold(&error) => Err(with_source(error.er_make(), source)),
            Err(source) => Err(context(error, source)),
        }
    }
}
#[cold]
#[inline(never)]
#[track_caller]
pub fn context<A, Mode, Input>(
    error: impl ErMake<A, Mode>,
    source: impl ErInput<Input>,
) -> ErTree<A>
where
    A: Error + 'static,
{
    build_tree(error.er_make(), source)
}

#[cold]
#[inline(never)]
#[track_caller]
pub fn context_with<A, Mode, Input, E: ErInput<Input>>(
    fields: impl ErWithFields<E::Error, A, Mode>,
    source: E,
) -> ErTree<A>
where
    A: Error + 'static,
{
    let error = fields.er_with_fields(source.er_input_error());
    build_tree(error, source)
}

#[cold]
#[inline(never)]
#[track_caller]
pub fn context_with_tree<A, Mode, Input, E: ErInput<Input>>(
    fields: impl ErWithFields<E, A, Mode>,
    source: E,
) -> ErTree<A>
where
    A: Error + 'static,
{
    let error = fields.er_with_fields(&source);
    build_tree(error, source)
}

#[cold]
#[inline(never)]
#[track_caller]
fn missing<A, Mode>(error: impl ErMake<A, Mode>) -> ErTree<A>
where
    A: Error + 'static,
{
    error.er_make().into_tree()
}

impl<T, E: ErInput<Input>, Mode, Input> ErPollExt<Mode, Input> for Poll<Result<T, E>> {
    type Ready<A> = ErResult<T, A>;

    #[track_caller]
    fn er<A>(self, error: impl ErMake<A, Mode>) -> Poll<ErResult<T, A>>
    where
        A: Error + 'static,
    {
        match self {
            Poll::Pending => Poll::Pending,
            // Returning Ok here saves a copy of the whole ErResult.
            Poll::Ready(Ok(value)) => Poll::Ready(Ok(value)),
            Poll::Ready(err @ Err(_)) => Poll::Ready(err.er(error)),
        }
    }
}
impl<T, E: ErInput<Input>, Mode, Input> ErPollExt<Mode, Input> for Poll<Option<Result<T, E>>> {
    type Ready<A> = Option<ErResult<T, A>>;

    #[track_caller]
    fn er<A>(self, error: impl ErMake<A, Mode>) -> Poll<Option<ErResult<T, A>>>
    where
        A: Error + 'static,
    {
        match self {
            Poll::Pending => Poll::Pending,
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Ready(Some(Ok(value))) => Poll::Ready(Some(Ok(value))),
            Poll::Ready(Some(err @ Err(_))) => Poll::Ready(Some(err.er(error))),
        }
    }
}

impl<T, E: ErInput<Input>, Input> ErResultExt<Input> for Result<T, E> {
    type Ok = T;
    type Err = E;

    fn er_result_input(self) -> Result<T, E> {
        self
    }
}
impl<T, E> ErValueExt for Result<T, E> {
    type Ok = T;
    type Err = E;

    fn er_value_input(self) -> Result<T, E> {
        self
    }
}
impl<T, E: IntoErTree> ErPresentationExt for Result<T, E> {
    type Ok = T;
    type Err = E::Error;

    fn er_tree(self) -> ErResult<T, E::Error> {
        match self {
            Ok(value) => Ok(value),
            Err(error) => Err(error.into_er_tree()),
        }
    }
}

impl<T, E: IntoErSnapshot<Input>, Input> ErSnapshotPresentationExt<Input> for Result<T, E> {
    type Ok = T;

    fn er_report(self) -> Result<T, String> {
        match self {
            Ok(value) => Ok(value),
            Err(error) => Err(error.into_er_report_text()),
        }
    }

    fn er_snapshot(self) -> Result<T, ErSnapshot> {
        match self {
            Ok(value) => Ok(value),
            Err(error) => Err(error.into_er_snapshot()),
        }
    }
}

impl<'a, T, E: IntoErTreeRef<'a>> ErBorrowedPresentationExt<'a> for Result<T, E> {
    type Ok = T;
    type Err = E::Error;

    fn er_tree(self) -> Result<T, &'a ErTree<E::Error>> {
        match self {
            Ok(value) => Ok(value),
            Err(error) => Err(error.into_er_tree_ref()),
        }
    }

    fn er_report<Input>(self) -> Result<T, String>
    where
        E::Error: ErReportError<Input>,
    {
        match self {
            Ok(value) => Ok(value),
            Err(error) => Err(error.into_er_tree_ref().er_report()),
        }
    }

    fn er_snapshot<Input>(self) -> Result<T, ErSnapshot>
    where
        E::Error: ErReportError<Input>,
    {
        match self {
            Ok(value) => Ok(value),
            Err(error) => Err(error.into_er_tree_ref().er_snapshot()),
        }
    }
}

impl<T, Mode> ErContextExt<Mode> for Option<T> {
    type Ok = T;

    #[track_caller]
    fn er<A>(self, error: impl ErMake<A, Mode>) -> ErResult<T, A>
    where
        A: Error + 'static,
    {
        match self {
            Some(value) => Ok(value),
            None if !er_make_cold(&error) => Err(error.er_make().into_tree()),
            None => Err(missing(error)),
        }
    }
}

#[cfg(feature = "macros")]
impl<T, F: Into<T>> ErBail<T, ErReady> for F {
    #[track_caller]
    fn er_bail(self) -> T {
        self.into()
    }
}
#[cfg(feature = "macros")]
impl<T, F: FnOnce() -> E, E: Into<T>> ErBail<T, (ErReady,)> for F {
    #[track_caller]
    fn er_bail(self) -> T {
        self().into()
    }
}
#[cfg(feature = "macros")]
impl<T, F, P, Form: ErForm> ErBail<T, (Form,)> for F
where
    T: ErBailTarget + From<ErTree<T::Error>>,
    T::Error: Error + ErBuildFrom<P, Form> + 'static,
    F: FnOnce(()) -> P,
{
    #[track_caller]
    fn er_bail(self) -> T {
        ErTree::from(T::Error::er_build_from(self(()))).into()
    }
}

#[cfg(feature = "macros")]
impl<T, P: core::fmt::Debug, Form: ErValueForm> ErBail<T, ((Form,),)> for P
where
    T: ErBailTarget + From<ErTree<T::Error>>,
    T::Error: Error + ErFromFields<P, Form> + 'static,
{
    #[inline]
    #[track_caller]
    fn er_bail(self) -> T {
        ErTree::from(T::Error::er_from_fields(self)).into()
    }
}

#[cfg(feature = "macros")]
impl<T, P, Form: ErForm> ErBailFields<T, Form> for P
where
    ErFieldInput<P>: ErFieldsFor<T::Error, Form>,
    T: ErBailTarget + From<ErTree<T::Error>>,
    T::Error: Error + ErBuildFrom<P, Form> + 'static,
{
    #[inline]
    #[track_caller]
    fn er_bail_fields(self) -> T {
        ErTree::from(T::Error::er_build_from(self)).into()
    }
}

#[cfg(feature = "macros")]
impl<T, P, Nodes, Form: ErForm> ErBail<T, (ErWithSaves, Form)> for ErErrorWithSaved<P, Nodes>
where
    Nodes: Into<Vec<ErNode>>,
    T: ErBailTarget + From<ErTree<T::Error>>,
    T::Error: Error + ErBuildFrom<P, Form> + 'static,
{
    #[cold]
    #[inline(never)]
    #[track_caller]
    fn er_bail(self) -> T {
        self.build::<T::Error, Form>().into_tree().into()
    }
}
#[cfg(feature = "macros")]
impl<T, P, Nodes, Form: ErForm> ErBailFields<T, (ErWithSaves, Form)> for ErErrorWithSaved<P, Nodes>
where
    ErFieldInput<P>: ErFieldsFor<T::Error, Form>,
    Nodes: Into<Vec<ErNode>>,
    T: ErBailTarget + From<ErTree<T::Error>>,
    T::Error: Error + ErBuildFrom<P, Form> + 'static,
{
    #[cold]
    #[inline(never)]
    #[track_caller]
    fn er_bail_fields(self) -> T {
        self.build::<T::Error, Form>().into_tree().into()
    }
}

impl<T, E: ErInput<Input>, Input> ErAllItem<(ErAllResult, Input)> for Result<T, E> {
    type Ok = T;

    #[track_caller]
    fn er_all_item(self) -> Result<T, ErNode> {
        match self {
            Ok(value) => Ok(value),
            Err(error) => Err(error.into_er_input()),
        }
    }
}

impl<'a, T, E, Input> ErAllItem<(ErAllResult, Input)> for &'a Result<T, E>
where
    &'a E: ErInput<Input>,
{
    type Ok = &'a T;

    #[track_caller]
    fn er_all_item(self) -> Result<Self::Ok, ErNode> {
        self.as_ref().er_all_item()
    }
}
impl<'a, T, E, Input> ErAllItem<(ErAllResult, Input)> for &'a mut Result<T, E>
where
    &'a E: ErInput<Input>,
{
    type Ok = &'a T;

    #[track_caller]
    fn er_all_item(self) -> Result<Self::Ok, ErNode> {
        (&*self).er_all_item()
    }
}
impl<T: ErAllItem<Mode>, Mode> ErAllItem<(ErAllOption, Mode)> for Option<T> {
    type Ok = Option<T::Ok>;

    #[track_caller]
    fn er_all_item(self) -> Result<Self::Ok, ErNode> {
        match self {
            Some(item) => item.er_all_item().map(Some),
            None => Ok(None),
        }
    }
}
impl<'a, T, Mode> ErAllItem<(ErAllOption, Mode)> for &'a Option<T>
where
    &'a T: ErAllItem<Mode>,
{
    type Ok = Option<<&'a T as ErAllItem<Mode>>::Ok>;

    #[track_caller]
    fn er_all_item(self) -> Result<Self::Ok, ErNode> {
        self.as_ref().er_all_item()
    }
}
impl<'a, T, Mode> ErAllItem<(ErAllOption, Mode)> for &'a mut Option<T>
where
    &'a T: ErAllItem<Mode>,
{
    type Ok = Option<<&'a T as ErAllItem<Mode>>::Ok>;

    #[track_caller]
    fn er_all_item(self) -> Result<Self::Ok, ErNode> {
        (&*self).er_all_item()
    }
}
impl<E: ErInput<Input>, Input> ErAllItem<Input> for E {
    type Ok = ();
    const ALWAYS_ERROR: bool = true;

    #[track_caller]
    fn er_all_item(self) -> Result<(), ErNode> {
        Err(self.into_er_input())
    }
}
