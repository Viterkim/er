use er::*;
use std::{error::Error, io};

pub trait ForeignSource {
    fn __er_as_source(&self) -> &'static dyn Error {
        &std::fmt::Error
    }
}
impl<T: ?Sized> ForeignSource for T {}

#[derive(Er)]
pub struct Parent {
    #[er(source)]
    pub cause: io::Error,
}

pub fn main() {}
