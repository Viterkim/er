use core::cell::Cell;
use core::error::Error;
use core::fmt;
use er::*;

pub struct NotSyncErr(pub Cell<u8>);
impl fmt::Display for NotSyncErr {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("not sync")
    }
}
impl fmt::Debug for NotSyncErr {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("NotSyncErr")
    }
}
impl Error for NotSyncErr {}

pub fn main() {
    let error = ErTree::from(NotSyncErr(Cell::new(0)));
    require_sync(&error);
    let _ = error.into_er_node();
}

pub fn require_sync<T: Sync>(_: &T) {}
