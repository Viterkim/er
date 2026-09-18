use crate::TestError;
use core::{error::Error, fmt};

impl fmt::Display for TestError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ErTest")
    }
}
impl fmt::Debug for TestError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, formatter)
    }
}
impl Error for TestError {}

impl From<()> for TestError {
    fn from(_: ()) -> Self {
        Self
    }
}
