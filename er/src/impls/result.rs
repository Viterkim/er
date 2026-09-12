use crate::{
    Er, ErError, ErOpaqueError, ErOption, ErPresentation, ErReport, ErResult, ErTop, ErTree,
    IntoErNode, IntoErTree,
};
use alloc::vec;
use core::error::Error;
#[cfg(feature = "src_locations")]
use core::panic::Location;

impl<T: Error + Sized + 'static> ErError for T {}

impl<T, E> ErResult for Result<T, E> {
    type Ok = T;
    type Err = E;

    #[cfg_attr(feature = "src_locations", track_caller)]
    fn er<A, F>(self, error: F) -> Er<T, A>
    where
        A: Error + 'static,
        F: FnOnce() -> A,
        E: IntoErNode,
    {
        match self {
            Ok(value) => Ok(value),
            Err(source) => {
                let error = error();
                let nodes = vec![source.into_er_node()];

                Err(ErTree {
                    top: error,
                    nodes,
                    #[cfg(feature = "src_locations")]
                    src_location: Location::caller(),
                })
            }
        }
    }

    #[cfg_attr(feature = "src_locations", track_caller)]
    fn er_from_val<A, F>(self, error: F) -> Er<T, A>
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

impl<T> ErOption for Option<T> {
    type Some = T;

    #[cfg_attr(feature = "src_locations", track_caller)]
    fn er<A, F>(self, error: F) -> Er<T, A>
    where
        A: Error + 'static,
        F: FnOnce() -> A,
    {
        match self {
            Some(value) => Ok(value),
            None => {
                let error = error();
                Err(ErTree::from(error))
            }
        }
    }
}

impl<T, E: IntoErTree> ErPresentation for Result<T, E> {
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

impl<T, E: ErOpaqueError> ErOpaqueError for Result<T, E> {
    type Output = Result<T, E::Output>;

    fn opaque_err(self) -> Self::Output {
        match self {
            Ok(value) => Ok(value),
            Err(error) => Err(error.opaque_err()),
        }
    }
}
