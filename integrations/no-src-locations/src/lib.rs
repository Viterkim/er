#![no_std]

use er::*;

#[derive(Er)]
pub struct DeviceErr;
pub fn diagnostic() -> ErTree<DeviceErr> {
    DeviceErr.er_add(DeviceErr).into()
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use core::panic::Location;

    #[test]
    pub fn captures_nothing() {
        let tree = diagnostic();

        assert!(tree.er_entries().all(|entry| entry.src_location.is_none()));
    }

    #[test]
    pub fn renders_supplied_location() {
        let mut tree = diagnostic();
        tree.nodes[0].src_location = Some(Location::caller());

        let report = tree.er_report();
        assert!(report.contains("`- DeviceErr @ "));
        assert_eq!(tree.er_snapshot().er_report(), report);
    }
}
