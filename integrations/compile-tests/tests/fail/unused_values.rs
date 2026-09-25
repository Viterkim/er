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
    tree.er_top().single_line();
    tree.er_entries();
    ErTree::from(AppErr);
    ErTree::from(AppErr).into_er_node();
    ErTree::from(AppErr).into_er_report();
    AppErrWrap { tree: ErTree::from(AppErr) };
    AppErr::new();
    Kind::missing();
    AppErr.er_wrap();
}
