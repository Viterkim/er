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
    let tree = AppErr.er();
    tree.er_report();
    tree.er_top().single_line();
    tree.er_entries();
    AppErr.er();
    AppErr.er().into_er_node();
    AppErr.er().into_er_report();
    AppErrWrap { tree: AppErr.er() };
    AppErr::new();
    Kind::missing();
    AppErr.er_wrap();
}
