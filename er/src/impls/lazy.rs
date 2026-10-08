use crate::ErLazyError;
use alloc::string::String;
use core::{error::Error, fmt};

impl fmt::Display for ErLazyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0.as_deref().unwrap_or("failed"))
    }
}
impl Error for ErLazyError {}
impl From<()> for ErLazyError {
    fn from((): ()) -> Self {
        Self(None)
    }
}
impl<P: Into<String>> From<(P,)> for ErLazyError {
    fn from((message,): (P,)) -> Self {
        Self(Some(message.into()))
    }
}
