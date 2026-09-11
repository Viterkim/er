use crate::{ErAsError, ErOpaqueError, ErReport, ErReportRef, ErTop, ErTopRef};
use core::{error::Error, fmt};

impl<P: fmt::Display> fmt::Display for ErAsError<P> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, formatter)
    }
}

impl<P: fmt::Display> fmt::Debug for ErAsError<P> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, formatter)
    }
}

impl<P: fmt::Display> Error for ErAsError<P> {}

impl<E: Error + 'static> ErOpaqueError for ErReport<E> {
    type Output = ErAsError<Self>;

    fn opaque_err(self) -> Self::Output {
        ErAsError(self)
    }
}

impl<E: fmt::Display> ErOpaqueError for ErTop<E> {
    type Output = ErAsError<Self>;

    fn opaque_err(self) -> Self::Output {
        ErAsError(self)
    }
}

impl<E: Error + 'static> ErOpaqueError for ErReportRef<'_, E> {
    type Output = ErAsError<Self>;

    fn opaque_err(self) -> Self::Output {
        ErAsError(self)
    }
}

impl<E: fmt::Display> ErOpaqueError for ErTopRef<'_, E> {
    type Output = ErAsError<Self>;

    fn opaque_err(self) -> Self::Output {
        ErAsError(self)
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
