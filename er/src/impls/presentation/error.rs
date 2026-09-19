use crate::ErAsError;
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
