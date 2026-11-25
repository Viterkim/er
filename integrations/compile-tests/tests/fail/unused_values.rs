#![deny(unused_must_use)]

use er::*;

#[derive(Er)]
#[er(wrap)]
pub struct AppErr;

#[derive(Er)]
pub enum Kind {
    Missing,
}

pub fn main() {
    let tree = ErTree::from(AppErr);
    tree.er_report();
    tree.er_entries();
    ErTree::from(AppErr);
    AppErrWrap { tree: ErTree::from(AppErr) };
    AppErr::new();
    Kind::missing();
}
