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
    pub device: Rc<RefCell<Device>>,
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
    use alloc::{boxed::Box, string::ToString, vec::Vec};
    use core::{error::Error, fmt};
    use std::io;

    #[test]
    pub fn chain() {
        let device = Rc::new(RefCell::new(Device { port: 8 }));
        let value: ErResult<(), DeviceErr> = Err::<(), _>(Rc::clone(&device)).er_val(|v| (v, 85));

        assert!(Rc::ptr_eq(&device, &value.unwrap_err().top.device.value));

        let mut tree = diagnostic(&device).unwrap_err();
        let expected = tree.er_report_string();
        assert_eq!(
            Err::<(), _>(&mut tree).er_report_string().unwrap_err(),
            expected
        );
        assert_eq!(
            Err::<(), _>(&tree).er_report_string().unwrap_err(),
            tree.er_report_string()
        );
        let leaf = tree.er_find::<DeviceErr>().unwrap();

        assert!(Rc::ptr_eq(&device, &leaf.device.value));
        assert!(ErShared::ptr_eq(&tree.top.device, &leaf.device));
        leaf.device.borrow_mut().port = 9;
        assert_eq!(tree.top.device.borrow().port, 9);

        let expected = tree.er_report().single_line().to_string();
        let saved: Rc<SavedErr> = tree
            .into_er_report()
            .single_line()
            .er_into(|report| (report, Rc::clone(&device)));
        assert_eq!(saved.report, expected);
        assert_eq!(saved.kind.device.borrow().port, 9);
        assert!(Rc::ptr_eq(&device, &saved.device));

        let snapshot = diagnostic(&device).er_snapshot().unwrap_err();
        let report = snapshot.er_report().single_line().er_report_string();
        let mut saved = snapshot.into_er_top().single_line();
        assert_eq!(
            Err::<(), _>(&mut saved).er_report_string().unwrap_err(),
            report
        );
        assert_eq!(Err::<(), _>(saved).er_report_string().unwrap_err(), report);
        assert_eq!(Ok::<_, ErSnapshot>(85).expect_report("saved failure"), 85);

        let mut wrapped = BoundaryErrWrap::from(ErTree::new(
            BoundaryErr::new(&device),
            [DeviceErr::new(&device, 86)],
        ));
        let expected = wrapped.to_string();
        let mut lines = Vec::new();
        wrapped
            .for_each_line(|l| lines.push(l.to_string()))
            .unwrap();
        assert_eq!(lines.join("\n"), expected);
        assert_eq!(
            Err::<(), _>(&mut wrapped)
                .er_report()
                .unwrap_err()
                .to_string(),
            expected
        );

        let tree: ErTree<AppErr> = Err::<(), _>(wrapped)
            .er_with_wrap(|err| {
                er_built(AppErr {
                    device: err.device.clone(),
                })
            })
            .unwrap_err();
        assert!(ErShared::ptr_eq(
            &tree.top.device,
            &tree.er_find::<DeviceErr>().unwrap().device
        ));
    }

    #[test]
    pub fn collect() {
        let device = Rc::new(RefCell::new(Device { port: 8 }));
        let tree: ErTree<AppErr> =
            er_all!(|_| &device, [save(&device), diagnostic(&device)]).unwrap_err();

        assert_eq!(tree.er_find_all::<DeviceErr>().count(), 2);
        let tree: ErTree<AppErr> = tree
            .into_er_top()
            .er_with_tree::<AppErr, _>(|tree| tree.er_find::<DeviceErr>().unwrap().device.clone());
        let tree: ErTree<AppErr> = tree.into_er_report().er_with(|err| err.device.clone());
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
        let report: ThreadReport = std::thread::spawn(move || wrapped.er_into(|report| report))
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
            let tree: ErResult<(), AppErr> = result.er_with(|err| {
                assert_eq!(err.to_string(), fmt::Error.to_string());
                device
            });
            assert!(tree.unwrap_err().er_contains::<fmt::Error>());

            let tree: ErTree<AppErr> = make().er(|_| device);
            assert!(tree.er_contains::<fmt::Error>());
            let tree: ErTree<AppErr> = make().er_with(|err| {
                assert_eq!(err.to_string(), fmt::Error.to_string());
                device
            });
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
