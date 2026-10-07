#[cfg(er_unsync)]
use crate::ErBoxedInput;
use crate::impls::tree::with_source;
use crate::{
    BoxError, ErContextExt, ErErrorContextExt, ErErrorExt, ErInput, ErMake, ErOpaqueErrorExt,
    ErPresentationExt, ErReport, ErResult, ErResultExt, ErSnapshot, ErTop, ErTree, ErValueExt,
    IntoErTree,
};
#[cfg(feature = "macros")]
use crate::{ErAllError, ErAllItem, ErAllResult, ErBail, ErFields, ErPart, ErReady};
#[cfg(er_unsync)]
use alloc::boxed::Box;
use alloc::string::String;
use core::{error::Error, fmt};

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

    #[cfg_attr(feature = "src_locations", track_caller)]
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

    #[cfg_attr(feature = "src_locations", track_caller)]
    fn er_build_tree<A>(self, f: impl FnOnce(&E) -> A) -> ErResult<T, A>
    where
        A: Error + 'static,
        E: ErInput<Input>,
    {
        match self {
            Ok(value) => Ok(value),
            Err(source) => {
                let error = f(&source);
                Err(with_source(error, source))
            }
        }
    }
}
impl<T, E> ErValueExt for Result<T, E> {
    type Ok = T;
    type Err = E;

    #[cfg_attr(feature = "src_locations", track_caller)]
    fn er_val<A, F>(self, f: F) -> ErResult<T, A>
    where
        A: Error + 'static,
        F: FnOnce(E) -> A,
    {
        match self {
            Ok(value) => Ok(value),
            Err(failure) => {
                let error = f(failure);
                Err(ErTree::from(error))
            }
        }
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

    fn er_top(self) -> Result<T, ErTop<E::Error>> {
        match self {
            Ok(value) => Ok(value),
            Err(error) => Err(error.into_er_top()),
        }
    }

    fn er_report(self) -> Result<T, ErReport<E::Error>> {
        match self {
            Ok(value) => Ok(value),
            Err(error) => Err(error.into_er_report()),
        }
    }

    fn er_report_string(self) -> Result<T, String>
    where
        E::Error: Error + 'static,
    {
        match self {
            Ok(value) => Ok(value),
            Err(error) => Err(error.into_er_report().er_report_string()),
        }
    }

    fn er_top_string(self) -> Result<T, String>
    where
        E::Error: fmt::Display,
    {
        match self {
            Ok(value) => Ok(value),
            Err(error) => Err(error.into_er_top().er_top_string()),
        }
    }

    fn er_snapshot(self) -> Result<T, ErSnapshot>
    where
        E::Error: Error + 'static,
    {
        match self {
            Ok(value) => Ok(value),
            Err(error) => Err(error.into_er_tree().er_snapshot()),
        }
    }
}
impl<T, E: ErOpaqueErrorExt> ErOpaqueErrorExt for Result<T, E> {
    type Output = Result<T, E::Output>;

    fn opaque_err(self) -> Self::Output {
        match self {
            Ok(value) => Ok(value),
            Err(error) => Err(error.opaque_err()),
        }
    }
}

impl<T, Mode> ErContextExt<Mode> for Option<T> {
    type Ok = T;

    #[cfg_attr(feature = "src_locations", track_caller)]
    fn er<A>(self, error: impl ErMake<A, Mode>) -> ErResult<T, A>
    where
        A: Error + 'static,
    {
        match self {
            Some(value) => Ok(value),
            None => Err(ErTree::from(error.er_make())),
        }
    }
}

#[cfg(feature = "macros")]
impl<T, F: Into<T>> ErBail<T, ErReady> for F {
    #[cfg_attr(feature = "src_locations", track_caller)]
    fn er_bail(self) -> T {
        self.into()
    }
}
#[cfg(feature = "macros")]
impl<T, F: FnOnce() -> E, E: Into<T>> ErBail<T, (ErReady,)> for F {
    #[cfg_attr(feature = "src_locations", track_caller)]
    fn er_bail(self) -> T {
        self().into()
    }
}
#[cfg(feature = "macros")]
impl<T, F> ErBail<T, (ErFields,)> for F
where
    T: IntoErTree + From<ErTree<T::Error>>,
    T::Error: Error + 'static,
    F: ErMake<T::Error, ErFields>,
{
    #[cfg_attr(feature = "src_locations", track_caller)]
    fn er_bail(self) -> T {
        ErTree::from(self.er_make()).into()
    }
}

#[cfg(feature = "macros")]
impl<T, E: ErInput<Input>, Input> ErAllItem<(ErAllResult, Input)> for Result<T, E> {
    type Ok = T;

    #[cfg_attr(feature = "src_locations", track_caller)]
    fn er_all_item(self) -> Result<T, ErPart> {
        match self {
            Ok(value) => Ok(value),
            Err(error) => Err(error.into_er_input()),
        }
    }
}
#[cfg(feature = "macros")]
impl<E: ErInput<Input>, Input> ErAllItem<(ErAllError, Input)> for E {
    type Ok = ();

    #[cfg_attr(feature = "src_locations", track_caller)]
    fn er_all_item(self) -> Result<(), ErPart> {
        Err(self.into_er_input())
    }
}
