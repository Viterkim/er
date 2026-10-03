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
    let device = Device::new(85, Secret);

    FirmwareErr::new(device).er_wrap()
}

#[derive(Er)]
#[er(format = "{report}")]
pub struct FirmwareError {
    #[er(into_top)]
    pub kind: FirmwareErr,
    #[er(into_report_string)]
    pub report: alloc::string::String,
    #[er(into_snapshot)]
    pub snapshot: ErSnapshot,
}
pub fn saved_diagnostic() -> Result<(), FirmwareError> {
    Err::<(), _>(diagnostic()).er_into(|_| {})
}

#[cfg(target_has_atomic = "ptr")]
#[derive(Er)]
pub struct SharedErr {
    pub device: ErShared<Device>,
}
#[cfg(target_has_atomic = "ptr")]
pub fn shared_diagnostic() -> ErTree<SharedErr> {
    let error = ErTree::from(SharedErr::new(Device::new(85, Secret)));
    error.er_with(|old| SharedErr::new(&old.device))
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

        let errors: ErTree<FirmwareErr> = er_all!(
            || FirmwareErr::new(Device::new(85, Secret)),
            ["bad".parse::<u8>().unwrap_err(), "bad".parse::<bool>()]
        )
        .unwrap_err();
        assert_eq!(errors.nodes.len(), 2);
        let errors = er_add!(errors, [core::fmt::Error, "bad".parse::<u16>()]);
        assert_eq!(errors.nodes.len(), 4);

        let values = er_try!(
            || FirmwareErr::new(Device::new(85, Secret)),
            ["85".parse::<u8>(), "true".parse::<bool>()],
        )
        .unwrap_report();
        assert_eq!(values, (85, true));

        let saved = saved_diagnostic().unwrap_err();
        assert_eq!(saved.report, saved.snapshot.er_report().to_string());
        assert_eq!(saved.kind.device.bus, 85);

        #[cfg(target_has_atomic = "ptr")]
        {
            let shared = shared_diagnostic();
            let old = shared.nodes[0].er_find::<SharedErr>().unwrap();
            assert_eq!(shared.top.device.bus, 85);
            assert!(ErShared::ptr_eq(&shared.top.device, &old.device));
        }
    }
}
