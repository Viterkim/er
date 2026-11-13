#[cfg(er_unsync)]
use crate::ErBoxedInput;
use crate::impls::tree::with_source;
use crate::{
    BoxError, ErBorrowedPresentationExt, ErContextExt, ErErrorContextExt, ErErrorExt, ErInput,
    ErMake, ErPresentationExt, ErReportError, ErResult, ErResultExt, ErSnapshot,
    ErSnapshotPresentationExt, ErTree, ErValueExt, IntoErSnapshot, IntoErTree, IntoErTreeRef,
};
use crate::{ErAllItem, ErAllOption, ErAllResult, ErPart};
#[cfg(feature = "macros")]
use crate::{
    ErBail, ErBailFields, ErBuildFrom, ErErrorWithSaved, ErForm, ErFromFields, ErReady,
    ErValueForm, ErWithSaves,
};
#[cfg(er_unsync)]
use alloc::boxed::Box;
use alloc::string::String;
use core::error::Error;

impl<T: Into<BoxError>, Mode> ErErrorContextExt<Mode> for T {}
impl<T: Into<BoxError>> ErErrorExt for T {}

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
            Err(source) => Err(with_source(error.er_make(), source)),
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
            None => Err(error.er_make().into_tree()),
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
    T: IntoErTree + From<ErTree<T::Error>>,
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
    T: IntoErTree + From<ErTree<T::Error>>,
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
    T: IntoErTree + From<ErTree<T::Error>>,
    T::Error: Error + ErBuildFrom<P, Form> + 'static,
{
    #[inline]
    #[track_caller]
    fn er_bail_fields(self) -> T {
        ErTree::from(T::Error::er_build_from(self)).into()
    }
}

#[cfg(feature = "macros")]
impl<T, P, Form: ErForm> ErBail<T, (ErWithSaves, Form)> for ErErrorWithSaved<P>
where
    T: IntoErTree + From<ErTree<T::Error>>,
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
impl<T, P, Form: ErForm> ErBailFields<T, (ErWithSaves, Form)> for ErErrorWithSaved<P>
where
    T: IntoErTree + From<ErTree<T::Error>>,
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
    fn er_all_item(self) -> Result<T, ErPart> {
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
    fn er_all_item(self) -> Result<Self::Ok, ErPart> {
        self.as_ref().er_all_item()
    }
}
impl<'a, T, E, Input> ErAllItem<(ErAllResult, Input)> for &'a mut Result<T, E>
where
    &'a E: ErInput<Input>,
{
    type Ok = &'a T;

    #[track_caller]
    fn er_all_item(self) -> Result<Self::Ok, ErPart> {
        (&*self).er_all_item()
    }
}
impl<T: ErAllItem<Mode>, Mode> ErAllItem<(ErAllOption, Mode)> for Option<T> {
    type Ok = Option<T::Ok>;

    #[track_caller]
    fn er_all_item(self) -> Result<Self::Ok, ErPart> {
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
    fn er_all_item(self) -> Result<Self::Ok, ErPart> {
        self.as_ref().er_all_item()
    }
}
impl<'a, T, Mode> ErAllItem<(ErAllOption, Mode)> for &'a mut Option<T>
where
    &'a T: ErAllItem<Mode>,
{
    type Ok = Option<<&'a T as ErAllItem<Mode>>::Ok>;

    #[track_caller]
    fn er_all_item(self) -> Result<Self::Ok, ErPart> {
        (&*self).er_all_item()
    }
}
impl<E: ErInput<Input>, Input> ErAllItem<Input> for E {
    type Ok = ();
    const ALWAYS_ERROR: bool = true;

    #[track_caller]
    fn er_all_item(self) -> Result<(), ErPart> {
        Err(self.into_er_input())
    }
}
