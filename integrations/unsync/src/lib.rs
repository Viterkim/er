#![no_std]

extern crate alloc;

use alloc::{rc::Rc, string::String};
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

#[derive(Er)]
#[er(format = "{report}")]
pub struct SavedErr {
    #[er(into_top)]
    pub kind: AppErr,
    #[er(into_report_string)]
    pub report: String,
}

pub fn save(device: &Rc<RefCell<Device>>) -> ErResult<(), SaveErr> {
    Err::<(), _>(DeviceErr::new(device, 85)).er(|_| device)
}

pub fn diagnostic(device: &Rc<RefCell<Device>>) -> ErResult<(), AppErr> {
    let result = save(device).map_err(SaveErrWrap::from);
    let result = result.er_with(|old| BoundaryErr::new(&old.device));
    let result = result.map_err(BoundaryErrWrap::from);

    result.er_wrap(|_| device)
}

#[cfg(test)]
extern crate std;

#[cfg(test)]
pub mod tests {
    use super::*;
    use alloc::{boxed::Box, string::ToString, vec::Vec};
    use core::{error::Error, fmt};
    use std::io;

    #[test]
    pub fn chain() {
        let device = Rc::new(RefCell::new(Device { port: 8 }));
        let tree = diagnostic(&device).unwrap_err();
        let leaf = tree.er_find::<DeviceErr>().unwrap();

        assert!(Rc::ptr_eq(&device, &leaf.device.value));
        assert!(ErShared::ptr_eq(&tree.top.device, &leaf.device));
        leaf.device.borrow_mut().port = 9;
        assert_eq!(tree.top.device.borrow().port, 9);

        let expected = tree.er_report_string();
        let saved: Rc<SavedErr> = tree.er_into(|_| {});
        assert_eq!(saved.report, expected);
        assert_eq!(saved.kind.device.borrow().port, 9);
    }

    #[test]
    pub fn collect() {
        let device = Rc::new(RefCell::new(Device { port: 8 }));
        let tree: ErTree<AppErr> = er_all!(
            || AppErr::new(&device),
            [save(&device), diagnostic(&device)]
        )
        .unwrap_err();

        assert_eq!(tree.er_find_all::<DeviceErr>().count(), 2);
        let tree = tree
            .into_er_top()
            .er_with_tree(|old| AppErr::new(&old.er_find::<DeviceErr>().unwrap().device));
        let tree = tree
            .into_er_report()
            .er_with(|old| AppErr::new(&old.device));
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
        use threaded::{ErTraceExt as _, IntoErTree as _};

        #[derive(threaded::Er)]
        #[er(wrap(output = report))]
        pub struct ThreadErr;

        #[derive(threaded::Er)]
        pub struct ThreadReport {
            #[er(into_report_string)]
            pub report: String,
        }

        let tree = threaded::ErTree::new(ThreadErr, [fmt::Error]).er_trace();
        let wrapped = ThreadErrWrap::from(tree);
        let report: ThreadReport = std::thread::spawn(move || wrapped.er_into(|_| {}))
            .join()
            .unwrap();
        assert!(report.report.contains("ThreadErr"));

        let device = Rc::new(RefCell::new(Device { port: 8 }));
        assert!(diagnostic(&device).unwrap_err().er_contains::<DeviceErr>());
    }

    #[test]
    pub fn boxed() {
        let device = Rc::new(RefCell::new(Device { port: 8 }));
        let cause = Box::new(io::Error::from(io::ErrorKind::PermissionDenied));
        let original = &*cause as *const io::Error;
        let both: Result<(), Box<dyn Error + Send + Sync>> = Err(cause);
        let line = line!() + 1;
        let tree: ErTree<AppErr> = both.er(|_| &device).unwrap_err();
        assert!(core::ptr::eq(
            tree.er_find::<io::Error>().unwrap(),
            original
        ));
        assert_eq!(tree.src_location.line(), line);
        assert_eq!(tree.nodes[0].src_location.line(), line);

        let both: Result<_, Box<dyn Error + Send + Sync>> = Ok(Rc::clone(&device));
        let result: ErResult<_, AppErr> = both.er(|| panic!("already Ok"));
        assert!(result.is_ok_and(|value| Rc::ptr_eq(&value, &device)));

        fn check<Input, E>(make: impl Fn() -> E, device: &Rc<RefCell<Device>>)
        where
            E: IntoErPart<Input, Error = E>
                + ErErrorContextExt<ErFields, Input>
                + ErErrorExt<Input>
                + fmt::Display,
        {
            let result = Err::<(), _>(make());
            let tree = result.er_with(|old| {
                assert_eq!(old.to_string(), fmt::Error.to_string());
                AppErr::new(device)
            });
            assert!(tree.unwrap_err().er_contains::<fmt::Error>());

            let tree: ErTree<AppErr> = make().er(|_| device);
            assert!(tree.er_contains::<fmt::Error>());
            let tree = make().er_with(|_| AppErr::new(device));
            assert!(tree.er_contains::<fmt::Error>());
            assert!(make().into_er_part().node.er_find::<fmt::Error>().is_some());

            let result = er_all!(|_| device, [make(), Err::<(), _>(make())]);
            let tree: ErTree<AppErr> = result.unwrap_err();
            assert_eq!(tree.er_find_all::<fmt::Error>().count(), 2);

            let results = [Ok(7), Err(make()), Err(make())];
            let result: ErResult<Vec<_>, AppErr> = results.into_iter().er_collect_all(|_| device);
            assert_eq!(result.unwrap_err().er_find_all::<fmt::Error>().count(), 2);

            let failure = Err::<(), _>(make()).er_test().unwrap_err();
            assert!(failure.er_contains::<fmt::Error>());
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

        fn bail<E, Mode>(error: E) -> ErTest
        where
            E: ErBail<ErTestFailure, Mode>,
        {
            er_bail!(error);
        }

        fn boxed() -> Box<dyn Error + Send + Sync> {
            Box::new(fmt::Error)
        }

        assert!(bail(boxed()).unwrap_err().er_contains::<fmt::Error>());
        assert!(bail(boxed).unwrap_err().er_contains::<fmt::Error>());
        assert!(bail(|_| boxed()).unwrap_err().er_contains::<fmt::Error>());
    }
}
