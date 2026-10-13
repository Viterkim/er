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
    let _ = Boundary::closed();
}
