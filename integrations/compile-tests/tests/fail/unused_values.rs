#![deny(unused_must_use)]

use er::*;

#[derive(Er)]
#[er(wrap)]
pub struct AppEr;

#[derive(Er)]
pub enum Kind {
    Missing,
}

pub fn main() {
    let tree = AppEr.er();
    tree.er_report();
    tree.er_top().single_line();
    tree.er_entries();
    AppEr.er();
    AppEr.er().into_er_node();
    AppEr.er().into_er_report();
    AppErWrap { tree: AppEr.er() };
    AppEr::new();
    Kind::missing();
    AppEr.er_wrap();
}
