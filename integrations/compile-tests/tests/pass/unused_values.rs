#![deny(unused_must_use)]

use er::*;

#[derive(Er)]
#[er(wrap)]
pub struct AppEr;

pub fn main() -> core::fmt::Result {
    let tree = AppEr.er();
    let _ = tree.er_report();
    let _ = tree.er_top().single_line();
    let _ = tree.er_entries();
    let _ = AppEr.er();
    let _ = AppEr::new();
    let _ = AppErWrap::from(AppEr.er());
    tree.er_report().for_each_line(|_| {})?;
    tree.er_top().for_each_line(|_| {})?;
    tree.er_for_each_entry(|_| {});
    tree.er_report().er_for_each_entry(|_| {});

    Ok(())
}
