use core::ptr;
use er::*;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

#[derive(Er)]
pub struct InnerErr;

#[derive(Er)]
pub struct Tracked {
    pub value: u8,
    #[er(skip)]
    pub drops: Arc<AtomicUsize>,
}
impl Drop for Tracked {
    fn drop(&mut self) {
        self.drops.fetch_add(1, Ordering::Relaxed);
    }
}

#[derive(Er)]
#[er(wrap)]
pub struct HandlerErr;

pub fn parse_wrapped(input: &str) -> Result<u16, HandlerErrWrap> {
    let port: u16 = input.parse().er(())?;
    Ok(port)
}

#[test]
pub fn conversions() {
    let drops = Arc::new(AtomicUsize::new(0));
    let child_tree = ErTree::new(
        Tracked::new(85, Arc::clone(&drops)),
        [Tracked::new(85, Arc::clone(&drops))],
    );
    let error = ErTree::new(HandlerErr, [child_tree]);
    let expected = error.er_report().to_string();
    let child_ptr = error.nodes[0].error.downcast_ref::<Tracked>().unwrap() as *const Tracked;
    let grandchild_ptr = error.nodes[0].nodes[0]
        .error
        .downcast_ref::<Tracked>()
        .unwrap() as *const Tracked;
    #[cfg(feature = "src_locations")]
    let locations = (
        error.src_location,
        error.nodes[0].src_location,
        error.nodes[0].nodes[0].src_location,
    );

    let wrapped = HandlerErrWrap::from(error);
    assert_eq!(wrapped.er_top().to_string(), "HandlerErr");

    let report: ErReport<HandlerErr> = wrapped.into();
    assert_eq!(report.to_string(), expected);

    fn propagate(result: Result<(), HandlerErrWrap>) -> Er<(), HandlerErr> {
        result?;
        Ok(())
    }

    let recovered = propagate(Err(report.into())).unwrap_err();
    assert!(ptr::eq(
        child_ptr,
        recovered.nodes[0].error.downcast_ref::<Tracked>().unwrap()
    ));
    assert!(ptr::eq(
        grandchild_ptr,
        recovered.nodes[0].nodes[0]
            .error
            .downcast_ref::<Tracked>()
            .unwrap()
    ));
    #[cfg(feature = "src_locations")]
    assert_eq!(
        (
            recovered.src_location,
            recovered.nodes[0].src_location,
            recovered.nodes[0].nodes[0].src_location
        ),
        locations
    );
    assert_eq!(drops.load(Ordering::Relaxed), 0);
    drop(recovered);
    assert_eq!(drops.load(Ordering::Relaxed), 2);
}

#[test]
pub fn wrapped_results() {
    assert_eq!(parse_wrapped("85").er_report().unwrap(), 85);
    assert_eq!(parse_wrapped("80").er_top().expect("usable response"), 80);

    let report = parse_wrapped("aint_even_a_number_cmon_man")
        .er_report()
        .unwrap_err();
    assert!(report.er_contains::<std::num::ParseIntError>());
    assert!(report.er_contains::<HandlerErr>());
}

#[test]
pub fn fresh_conversions() {
    let _line = line!() + 1;
    let wrapped = HandlerErr::new().er_wrap();

    assert!(wrapped.tree.nodes.is_empty());
    #[cfg(feature = "src_locations")]
    assert_eq!(
        (
            wrapped.tree.src_location.file(),
            wrapped.tree.src_location.line()
        ),
        (file!(), _line)
    );

    let _line = line!() + 1;
    let wrapped: HandlerErrWrap = HandlerErr::new().into();

    #[cfg(feature = "src_locations")]
    assert_eq!(wrapped.tree.src_location.line(), _line);
    assert_eq!(wrapped.er_top().to_string(), "HandlerErr");

    fn from_result(line: &mut u32) -> Result<(), HandlerErrWrap> {
        let result: Result<(), HandlerErr> = Err(HandlerErr::new());
        *line = line!() + 1;
        result?;
        Ok(())
    }

    let mut _line = 0;
    let wrapped = from_result(&mut _line).unwrap_err();

    #[cfg(feature = "src_locations")]
    assert_eq!(wrapped.tree.src_location.line(), _line);
    assert!(wrapped.tree.nodes.is_empty());
}

#[derive(Er)]
#[er(wrap(name = ApiError))]
pub struct RenamedErr;
#[test]
pub fn custom_name() {
    let error = ErTree::new(RenamedErr, [InnerErr]);
    let wrapped = ApiError::from(error);

    assert!(wrapped.er_contains::<InnerErr>());
}

#[derive(Er)]
#[er(wrap(output = top))]
pub struct TopOutputErr;
#[test]
pub fn top_output() {
    let error = ErTree::new(TopOutputErr, [InnerErr]);
    #[cfg(feature = "src_locations")]
    let location = error.src_location;
    let wrapped = TopOutputErrWrap::from(error);

    assert_eq!(wrapped.to_string(), "TopOutputErr");
    assert_eq!(format!("{wrapped:?}"), "TopOutputErr");

    let result: Result<(), TopOutputErrWrap> = Err(wrapped);
    let report = result.er::<HandlerErr>(()).er_report().unwrap_err();

    assert!(report.er_contains::<TopOutputErr>());
    assert!(report.er_contains::<InnerErr>());
    #[cfg(feature = "src_locations")]
    assert_eq!(report.tree.nodes[0].src_location, location);
}

#[derive(Er)]
#[er(wrap(output = report))]
pub struct ReportOutputErr;
#[test]
pub fn report_output() {
    let error = ErTree::new(ReportOutputErr, [InnerErr]);
    let wrapped = ReportOutputErrWrap::from(error);

    let displayed = wrapped.to_string();
    assert!(displayed.contains("ReportOutputErr"), "{displayed}");
    assert!(displayed.contains("InnerErr"), "{displayed}");
    assert_eq!(format!("{wrapped:?}"), displayed);

    let result: Result<(), ReportOutputErrWrap> = Err(wrapped);
    let outer = result.er::<HandlerErr>(()).unwrap_err();
    assert!(outer.er_contains::<ReportOutputErr>());
    assert!(outer.er_contains::<InnerErr>());
}

#[test]
pub fn report_reentry() {
    let tree = ErTree::new(ReportOutputErr, [std::io::Error::other("disk")]);
    let child = tree.er_find::<std::io::Error>().unwrap() as *const std::io::Error;
    let report = tree.into_er_report().single_line();

    let result: Result<(), ErReport<ReportOutputErr>> = Err(report);
    let outer = result.er::<HandlerErr>(()).unwrap_err();

    assert!(outer.er_contains::<ReportOutputErr>());
    assert!(ptr::eq(child, outer.er_find::<std::io::Error>().unwrap()));
}

#[derive(Er)]
#[er(wrap(output = report, std_error))]
pub struct StandardErr<T>(pub T);

#[test]
pub fn std_error() {
    let drops = Arc::new(AtomicUsize::new(0));
    let tree = ErTree::new(
        StandardErr::new(85u8),
        [
            Tracked::new(1, Arc::clone(&drops)),
            Tracked::new(2, Arc::clone(&drops)),
        ],
    );
    let expected = tree.er_report().to_string();
    let child = &*tree.nodes[0].error as *const _;
    #[cfg(feature = "src_locations")]
    let location = tree.src_location;
    let wrapped = StandardErrWrap::from(tree);

    assert_eq!(wrapped.to_string(), expected);
    assert_eq!(format!("{wrapped:?}"), expected);

    let _line = line!() + 1;
    let result = Err::<(), _>(wrapped).er_from_wrap(HandlerErr::new);
    let tree = result.unwrap_err();

    assert_eq!(tree.er_find_all::<Tracked>().count(), 2);
    assert!(ptr::eq(child, &*tree.nodes[0].nodes[0].error));
    #[cfg(feature = "src_locations")]
    {
        assert_eq!(tree.src_location.line(), _line);
        assert_eq!(tree.nodes[0].src_location, location);
    }
    assert_eq!(drops.load(Ordering::Relaxed), 0);
    drop(tree);
    assert_eq!(drops.load(Ordering::Relaxed), 2);

    let wrapped = StandardErrWrap::from(ErTree::new(StandardErr::new(85u8), [InnerErr]));
    let expected = wrapped.to_string();
    let outer = Err::<(), _>(wrapped).er::<HandlerErr>(()).unwrap_err();

    assert!(outer.er_contains::<StandardErrWrap<u8>>());
    assert_eq!(outer.nodes[0].error.to_string(), expected);

    let mut called = false;
    let success = Ok::<_, StandardErrWrap<u8>>(7).er_from_wrap(|| {
        called = true;
        HandlerErr
    });

    assert_eq!(success.ok(), Some(7));
    assert!(!called);
}

#[derive(Er)]
#[er(wrap(name = GenericWrap, output = report))]
pub struct GenericErr<T> {
    pub value: T,
}
#[test]
pub fn generic_wrap() {
    pub fn roundtrip<T>(tree: ErTree<GenericErr<T>>) -> ErTree<GenericErr<T>> {
        let wrapped = GenericWrap::from(tree);
        let report: ErReport<GenericErr<T>> = wrapped.into();
        let wrapped = GenericWrap::from(report);
        let result: Result<(), GenericWrap<T>> = Err(wrapped);
        let top = result.er_top().unwrap_err();
        GenericWrap::from(top).into_er_tree()
    }

    let error = GenericErr { value: 7u8 }.er();
    let wrapped = GenericWrap::from(roundtrip(error));

    assert!(wrapped.to_string().contains("GenericErr { value: 7 }"));
    assert_eq!(wrapped.tree.top.value, 7);
}

#[derive(Er)]
#[er(wrap(output = top))]
pub struct BorrowedErr<'a> {
    pub text: &'a str,
}
#[test]
pub fn borrowed_top() {
    let text = String::from("borrowed");
    let tree = ErTree {
        top: BorrowedErr::new(&text),
        nodes: Vec::new(),
        #[cfg(feature = "src_locations")]
        src_location: std::panic::Location::caller(),
    };
    let wrapped = BorrowedErrWrap::from(tree);

    assert_eq!(wrapped.to_string(), wrapped.er_top().to_string());
    assert_eq!(format!("{wrapped:?}"), "BorrowedErr { text: \"borrowed\" }");

    let report: ErReport<BorrowedErr<'_>> = wrapped.into();
    let tree: ErTree<BorrowedErr<'_>> = report.into();
    let wrapped = BorrowedErrWrap::from(tree);

    assert_eq!(wrapped.tree.top.text, text);

    let result: Result<(), BorrowedErrWrap<'_>> = Err(wrapped);
    assert_eq!(result.er_top().unwrap_err().tree.top.text, text);
}
