use er::*;

#[derive(Er)]
#[er(wrap)]
pub struct BoundaryErr;

pub trait BoundaryError: core::error::Error {
    fn status_code(&self) -> u16;
}
