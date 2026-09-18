#![deny(unused_must_use)]

use er::*;

#[derive(Er)]
#[er(wrap)]
pub struct AppErr;

pub fn main() -> core::fmt::Result {
    let tree = AppErr.er();
    let _ = tree.er_report();
    let _ = tree.er_top().single_line();
    let _ = tree.er_entries();
    let _ = AppErr.er();
    let _ = AppErr::new();
    let _ = AppErrWrap::from(AppErr.er());
    tree.er_report().for_each_line(|_| {})?;
    tree.er_top().for_each_line(|_| {})?;
    tree.er_for_each_entry(|_| {});
    tree.er_report().er_for_each_entry(|_| {});

    Ok(())
}
