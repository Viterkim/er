#![no_std]

use er::*;

#[derive(Er)]
pub struct DeviceErr;
pub fn diagnostic() -> ErTree<DeviceErr> {
    ErTree::new(DeviceErr, [DeviceErr])
}

#[cfg(test)]
pub mod tests {
    // The fields should be gone, not just empty.
    #[test]
    pub fn location_fields_are_absent() {
        trybuild::TestCases::new().compile_fail("tests/fail/*.rs");
    }
}
