#![no_std]

// The derives and Wrap must work without the std prelude.
extern crate alloc;

use er::*;

pub struct Secret;

#[derive(ErFormat)]
pub struct Device {
    pub bus: u8,
    #[er(censor)]
    pub key: Secret,
}

#[derive(Er)]
#[er(wrap)]
pub struct FirmwareErr {
    pub device: Device,
}
pub fn diagnostic() -> FirmwareErrWrap {
    let device = Device {
        bus: 85,
        key: Secret,
    };

    FirmwareErr::new(device).er_wrap()
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use alloc::string::ToString;

    #[test]
    pub fn downstream_no_std() {
        let error = diagnostic();
        assert_eq!(
            error.er_top().to_string(),
            "FirmwareErr { device: Device { bus: 85, key: *CENSORED* } }"
        );
    }
}
