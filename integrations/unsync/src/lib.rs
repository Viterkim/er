#![no_std]

extern crate alloc;

use alloc::rc::Rc;
use core::cell::RefCell;
use er::*;

#[derive(Debug)]
pub struct Device {
    pub port: u8,
}

#[derive(Er)]
pub struct DeviceErr {
    pub device: ErShared<RefCell<Device>>,
    pub code: u8,
}

#[derive(Er)]
#[er(wrap(output = report))]
pub struct SaveErr {
    pub device: ErShared<RefCell<Device>>,
}

#[derive(Er)]
#[er(wrap(output = report, std_error))]
pub struct BoundaryErr {
    pub device: ErShared<RefCell<Device>>,
}

#[derive(Er)]
pub struct AppErr {
    pub device: ErShared<RefCell<Device>>,
}

pub fn save(device: &Rc<RefCell<Device>>) -> ErResult<(), SaveErr> {
    Err::<(), _>(DeviceErr::new(device, 85)).er(|_| device)
}

pub fn diagnostic(device: &Rc<RefCell<Device>>) -> ErResult<(), AppErr> {
    let result = save(device).map_err(SaveErrWrap::from);
    let result = result.er_with::<BoundaryErr, _>(|err| err.device.clone());
    let result = result.map_err(BoundaryErrWrap::from);

    result.er_with_wrap(|err| err.device.clone())
}

#[cfg(test)]
extern crate std;

#[cfg(test)]
pub mod tests {
    use super::*;
    use alloc::{boxed::Box, string::String, vec::Vec};
    use core::{error::Error, fmt};

    #[test]
    pub fn chain() {
        let device = Rc::new(RefCell::new(Device { port: 8 }));
        let _: ErResult<(), DeviceErr> = Err::<(), _>(Rc::clone(&device)).er_val(|v| (v, 85));

        let tree = diagnostic(&device).unwrap_err();
        tree.er_find::<DeviceErr>()
            .unwrap()
            .device
            .borrow_mut()
            .port = 9;
        assert_eq!(tree.top.device.borrow().port, 9);

        let expected = tree.er_report();
        let (_, report) = tree.er_split();
        assert_eq!(report, expected);

        let wrapped = BoundaryErrWrap::from(ErTree::new(
            BoundaryErr::new(&device),
            [DeviceErr::new(&device, 86)],
        ));
        let tree: ErTree<AppErr> = Err::<(), _>(wrapped)
            .er_with_wrap(|err| AppErr {
                device: err.device.clone(),
            })
            .unwrap_err();
        assert!(tree.er_contains::<DeviceErr>());
    }

    #[test]
    pub fn collect() {
        let device = Rc::new(RefCell::new(Device { port: 8 }));
        let tree: ErTree<AppErr> =
            er_all!(|_| &device, [save(&device), diagnostic(&device)]).unwrap_err();

        assert_eq!(tree.er_find_all::<DeviceErr>().count(), 2);
        let tree: ErTree<AppErr> = tree
            .er_with_tree::<AppErr, _>(|tree| tree.er_find::<DeviceErr>().unwrap().device.clone());
        let tree: ErTree<AppErr> = tree.er_with(|err| err.device.clone());
        let node = tree.into_er_part();
        assert!(node.er_error().is::<AppErr>());

        let mut tree = ErTree::new(AppErr::new(&device), [node]);
        let boxed: BoxError = Box::new(DeviceErr::new(&device, 86));
        tree = er_add!(tree, [boxed, save(&device)]);
        assert_eq!(tree.er_find_all::<DeviceErr>().count(), 4);

        let errors: ErResult<Vec<()>, AppErr> = [save(&device), save(&device)]
            .into_iter()
            .er_collect_all(|_| &device);
        assert_eq!(errors.unwrap_err().er_find_all::<DeviceErr>().count(), 2);
    }

    #[test]
    pub fn coexist() {
        use threaded::IntoErTree as _;

        #[derive(threaded::Er)]
        #[er(crate = threaded, wrap(output = report))]
        pub struct ThreadErr;

        let tree = threaded::er_new!(ThreadErr, [fmt::Error]);
        let wrapped = ThreadErrWrap::from(tree);
        let (_, report) = std::thread::spawn(move || wrapped.er_split())
            .join()
            .unwrap();
        assert!(report.contains("ThreadErr"));

        let device = Rc::new(RefCell::new(Device { port: 8 }));
        assert!(diagnostic(&device).unwrap_err().er_contains::<DeviceErr>());
    }

    #[test]
    pub fn boxed() {
        let device = Rc::new(RefCell::new(Device { port: 8 }));
        let both: Result<_, Box<dyn Error + Send + Sync>> = Ok(Rc::clone(&device));
        let _: ErResult<_, AppErr> = both.er(|| -> AppErr { panic!("already Ok") });

        fn check<Input, E>(make: impl Fn() -> E, device: &Rc<RefCell<Device>>)
        where
            E: IntoErPart<Input, Error = E>
                + ErErrorContextExt<er::internal::ErFields, Input>
                + ErErrorExt<Input>
                + fmt::Display,
        {
            let tree: ErResult<(), AppErr> = Err::<(), _>(make()).er_with(|_| device);
            assert!(tree.unwrap_err().er_contains::<fmt::Error>());

            let _: ErTree<AppErr> = make().er(|_| device);
            let _: ErTree<AppErr> = make().er_with(|_| device);
            let _ = make().into_er_part();
            let _: ErResult<(), AppErr> = er_all!(|_| device, [make(), Err::<(), _>(make())]);
            let _: ErResult<Vec<_>, AppErr> =
                [Ok(7), Err(make())].into_iter().er_collect_all(|_| device);
            let _ = Err::<(), _>(make()).er_test();
        }

        check(|| -> Box<dyn Error> { Box::new(fmt::Error) }, &device);
        check(
            || -> Box<dyn Error + Send> { Box::new(fmt::Error) },
            &device,
        );
        check(
            || -> Box<dyn Error + Sync> { Box::new(fmt::Error) },
            &device,
        );
        check(
            || -> Box<dyn Error + Send + Sync> { Box::new(fmt::Error) },
            &device,
        );

        let result: Result<(), _> = Err((boxed(), String::from("driver report")));
        let tree: ErResult<(), AppErr> = result.er(&device);
        assert!(tree.unwrap_err().er_report().contains("driver report"));

        fn bail<E, Mode>(error: E) -> ErTest
        where
            E: er::internal::ErBail<ErFailure, Mode>,
        {
            er_bail!(error);
        }

        fn boxed() -> Box<dyn Error + Send + Sync> {
            Box::new(fmt::Error)
        }

        assert!(bail(boxed()).unwrap_err().er_contains::<fmt::Error>());
        let _ = bail(boxed);
        let _ = bail(|_| boxed());
    }
}
