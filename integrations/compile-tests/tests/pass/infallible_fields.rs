#![deny(unreachable_code)]

use er::*;
use std::convert::Infallible;

#[derive(Er)]
pub struct Impossible(pub Infallible);

#[derive(Er)]
pub enum Boundary {
    Closed,
    Impossible(#[er(source)] Infallible),
}

fn main() {
    assert_eq!(Boundary::closed().to_string(), "Boundary::Closed");
}
