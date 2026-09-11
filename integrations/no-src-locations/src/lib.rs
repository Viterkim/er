#![no_std]

use er::*;

#[derive(Er)]
pub struct DeviceEr;
pub fn diagnostic() -> ErTree<DeviceEr> {
    ErTree::new(DeviceEr, [DeviceEr])
}

#[cfg(test)]
pub mod tests {
    // The fields should be gone, not just empty.
    #[test]
    pub fn location_fields_are_absent() {
        trybuild::TestCases::new().compile_fail("tests/fail/*.rs");
    }
}
