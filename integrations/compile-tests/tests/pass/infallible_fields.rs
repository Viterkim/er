#![forbid(unreachable_code)]

use er::*;
use std::convert::Infallible;

#[derive(Er)]
pub struct Impossible(pub Infallible);

#[derive(Er)]
pub enum Boundary {
    Closed(u32),
    #[er(no_constructors)]
    Impossible(#[er(source)] Infallible),
}

macro_rules! stop {
    () => { return Self { code: 503, attempts: 0 } };
}

#[derive(Er)]
pub struct Retry {
    pub code: u32,
    #[er(default = stop!())]
    pub attempts: u8,
}

fn main() {
    let _ = Boundary::closed(404u32);
    let error: ErTree<Retry> = Err::<(), _>(core::fmt::Error).er(404u32).unwrap_err();
    assert_eq!(error.top.code, 503);
}
