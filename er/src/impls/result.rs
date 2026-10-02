use crate::{
    BoxError, ErContextExt, ErErrorContextExt, ErErrorExt, ErMake, ErOpaqueErrorExt,
    ErPresentationExt, ErReport, ErResult, ErResultExt, ErTop, ErTree, IntoErPart, IntoErTree,
};
#[cfg(feature = "macros")]
use crate::{ErAllError, ErAllItem, ErAllResult, ErBail, ErBuilt, ErPart};
use core::error::Error;

impl<T: Into<BoxError>, Mode> ErErrorContextExt<Mode> for T {}
impl<T: Into<BoxError>> ErErrorExt for T {}

impl<T, E: IntoErPart, Mode> ErContextExt<Mode> for Result<T, E> {
    type Ok = T;

    #[cfg_attr(feature = "src_locations", track_caller)]
    fn er<A>(self, error: impl ErMake<A, Mode>) -> ErResult<T, A>
    where
        A: Error + 'static,
    {
        match self {
            Ok(value) => Ok(value),
            Err(source) => Err(ErTree::from(error.er_make()).er_add([source])),
        }
    }
}
impl<T, E> ErResultExt for Result<T, E> {
    type Ok = T;
    type Err = E;

    #[cfg_attr(feature = "src_locations", track_caller)]
    fn er_with_tree<A>(self, f: impl FnOnce(&E) -> A) -> ErResult<T, A>
    where
        A: Error + 'static,
        E: IntoErPart,
    {
        match self {
            Ok(value) => Ok(value),
            Err(source) => {
                let tree = ErTree::from(f(&source));
                Err(tree.er_add([source]))
            }
        }
    }

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
            Err(error) => Err(error.into_er_tree().into_er_top()),
        }
    }

    fn er_report(self) -> Result<T, ErReport<E::Error>> {
        match self {
            Ok(value) => Ok(value),
            Err(error) => Err(error.into_er_tree().into_er_report()),
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
impl<T, F: Into<T>> ErBail<T, ErBuilt> for F {
    #[cfg_attr(feature = "src_locations", track_caller)]
    fn er_bail(self) -> T {
        self.into()
    }
}
#[cfg(feature = "macros")]
impl<T, F, Mode> ErBail<T, (Mode,)> for F
where
    T: IntoErTree + From<ErTree<T::Error>>,
    T::Error: Error + 'static,
    F: ErMake<T::Error, Mode>,
{
    #[cfg_attr(feature = "src_locations", track_caller)]
    fn er_bail(self) -> T {
        ErTree::from(self.er_make()).into()
    }
}

#[cfg(feature = "macros")]
impl<T, E: IntoErPart> ErAllItem<ErAllResult> for Result<T, E> {
    type Ok = T;

    #[cfg_attr(feature = "src_locations", track_caller)]
    fn er_all_item(self) -> Result<T, ErPart> {
        match self {
            Ok(value) => Ok(value),
            Err(error) => Err(error.into_er_part()),
        }
    }
}
#[cfg(feature = "macros")]
impl<E: IntoErPart> ErAllItem<ErAllError> for E {
    type Ok = ();

    #[cfg_attr(feature = "src_locations", track_caller)]
    fn er_all_item(self) -> Result<(), ErPart> {
        Err(self.into_er_part())
    }
}
