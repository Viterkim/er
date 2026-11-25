#![deny(deprecated)]

use er::{Er, ErTree};
use std::fmt;

#[derive(Er)]
pub enum Outer {
    #[allow(deprecated)]
    Failed(#[er(skip)] ErTree<fmt::Error>),
}

fn main() {}
