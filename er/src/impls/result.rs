use crate::{
    Er, ErContextExt, ErErrorExt, ErMake, ErOpaqueErrorExt, ErPresentationExt, ErReport,
    ErResultExt, ErTop, ErTree, IntoErPart, IntoErTree,
};
use core::error::Error;

impl<T: Error + Sized + 'static> ErErrorExt for T {}

impl<T, E: IntoErPart, Mode> ErContextExt<Mode> for Result<T, E> {
    type Ok = T;

    #[cfg_attr(feature = "src_locations", track_caller)]
    fn er<A>(self, error: impl ErMake<A, Mode>) -> Er<T, A>
    where
        A: Error + 'static,
    {
        match self {
            Ok(value) => Ok(value),
            Err(source) => Err(ErTree::from(error.er_make()).with_part(source.into_er_part())),
        }
    }
}
impl<T, E> ErResultExt for Result<T, E> {
    type Ok = T;
    type Err = E;

    #[cfg_attr(feature = "src_locations", track_caller)]
    fn er_with<A>(self, error: impl FnOnce(&E) -> A) -> Er<T, A>
    where
        A: Error + 'static,
        E: IntoErPart,
    {
        match self {
            Ok(value) => Ok(value),
            Err(source) => {
                let tree = ErTree::from(error(&source));
                Err(tree.with_part(source.into_er_part()))
            }
        }
    }

    #[cfg_attr(feature = "src_locations", track_caller)]
    fn er_val<A, F>(self, error: F) -> Er<T, A>
    where
        A: Error + 'static,
        F: FnOnce(E) -> A,
    {
        match self {
            Ok(value) => Ok(value),
            Err(failure) => {
                let error = error(failure);
                Err(ErTree::from(error))
            }
        }
    }
}
impl<T, E: IntoErTree> ErPresentationExt for Result<T, E> {
    type Ok = T;
    type Err = E::Error;

    fn er_tree(self) -> Er<T, E::Error> {
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
    fn er<A>(self, error: impl ErMake<A, Mode>) -> Er<T, A>
    where
        A: Error + 'static,
    {
        match self {
            Some(value) => Ok(value),
            None => Err(ErTree::from(error.er_make())),
        }
    }
}
