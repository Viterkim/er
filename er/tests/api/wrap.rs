use core::{error::Error, ptr};
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
pub fn failing() -> Er<(), HandlerErr> {
    Err(ErTree::new(
        HandlerErr,
        [ErTree::new(InnerErr, [core::fmt::Error])],
    ))
}

pub fn parse_wrapped(input: &str) -> Result<u16, HandlerErrWrap> {
    let port: u16 = input.parse().er(HandlerErr::new)?;
    Ok(port)
}

#[test]
pub fn conversions() {
    let drops = Arc::new(AtomicUsize::new(0));
    let child = ErTree::new(
        Tracked::new(85, Arc::clone(&drops)),
        [Tracked::new(85, Arc::clone(&drops))],
    );
    let error = ErTree::new(HandlerErr, [child]);
    let expected = error.er_report().to_string();
    let child = error.nodes[0].error.downcast_ref::<Tracked>().unwrap() as *const Tracked;
    let grandchild = error.nodes[0].nodes[0]
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

    let tree: ErTree<HandlerErr> = wrapped.into();
    let wrapped = HandlerErrWrap::from(tree);
    let report = wrapped.into_er_report().single_line();
    let wrapped = HandlerErrWrap::from(report);
    let top = wrapped.into_er_top().single_line();
    let wrapped = HandlerErrWrap::from(top);
    let report: ErReport<HandlerErr> = wrapped.into();
    assert_eq!(report.layout, Layout::Multiline);
    let tree: ErTree<HandlerErr> = report.into();
    let wrapped = HandlerErrWrap::from(tree);
    let top: ErTop<HandlerErr> = wrapped.into();
    assert_eq!(top.layout, Layout::Multiline);
    let tree: ErTree<HandlerErr> = top.into();

    pub fn propagate(result: Result<(), HandlerErrWrap>) -> Er<(), HandlerErr> {
        result?;

        Ok(())
    }

    let recovered = propagate(Err(tree.into())).unwrap_err();
    assert_eq!(recovered.er_report().to_string(), expected);
    assert!(ptr::eq(
        child,
        recovered.nodes[0].error.downcast_ref::<Tracked>().unwrap()
    ));
    assert!(ptr::eq(
        grandchild,
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

    for top in [false, true] {
        let wrapped = HandlerErrWrap::from(failing().unwrap_err());
        let expected = if top {
            wrapped.er_top().to_string()
        } else {
            wrapped.er_report().to_string()
        };
        let result: Result<(), HandlerErrWrap> = Err(wrapped);
        let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            if top {
                result.er_top().expect("request failed");
            } else {
                result.er_report().expect("request failed");
            }
        }))
        .unwrap_err();

        assert_eq!(
            panic.downcast_ref::<String>().unwrap(),
            &format!("request failed: {expected}")
        );
    }
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

    let _line = line!() + 1;
    let wrapped = HandlerErrWrap::from(HandlerErr::new());
    #[cfg(feature = "src_locations")]
    assert_eq!(wrapped.tree.src_location.line(), _line);
    assert!(wrapped.tree.nodes.is_empty());

    pub fn from_result(line: &mut u32) -> Result<(), HandlerErrWrap> {
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

    let tree: ErTree<HandlerErr> = HandlerErr::new().er();
    assert!(tree.nodes.is_empty());

    let wrapped = GenericErr::new(std::rc::Rc::new(std::cell::Cell::new(7))).er_wrap();
    assert_eq!(wrapped.tree.top.value.get(), 7);
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
    let wrapped = TopOutputErrWrap::from(error);

    assert_eq!(wrapped.to_string(), "TopOutputErr");
    assert_eq!(format!("{wrapped:?}"), "TopOutputErr");
    assert!(Error::source(&wrapped).is_none());

    let result: Result<(), TopOutputErrWrap> = Err(wrapped);
    let report = result.er_report().unwrap_err();

    assert!(report.er_contains::<TopOutputErr>());
    assert!(report.er_contains::<InnerErr>());
    assert!(!report.er_contains::<TopOutputErrWrap>());
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
    assert!(Error::source(&wrapped).is_none());

    let result: Result<(), ReportOutputErrWrap> = Err(wrapped);
    assert_eq!(result.er_top().unwrap_err().to_string(), "ReportOutputErr");
}

#[test]
pub fn opaque_reentry() {
    let error = ErTree::new(ReportOutputErr, [InnerErr]);
    let wrapped = ReportOutputErrWrap::from(error);

    let result: Result<(), ReportOutputErrWrap> = Err(wrapped);
    let outer = result.er(HandlerErr::new).unwrap_err();

    assert!(outer.er_contains::<ReportOutputErrWrap>());
    assert!(!outer.er_contains::<InnerErr>());

    let recovered = outer.er_find::<ReportOutputErrWrap>().unwrap();
    assert!(recovered.tree.er_contains::<InnerErr>());
}

#[test]
pub fn structural_reentry() {
    let tree = ErTree::new(ReportOutputErr, [InnerErr]);
    #[cfg(feature = "src_locations")]
    let location = tree.src_location;
    let result: Result<(), ReportOutputErrWrap> = Err(tree.into());
    let outer = result.er_tree().er(HandlerErr::new).unwrap_err();

    assert!(outer.er_contains::<ReportOutputErr>());
    assert!(outer.er_contains::<InnerErr>());
    assert!(!outer.er_contains::<ReportOutputErrWrap>());
    #[cfg(feature = "src_locations")]
    assert_eq!(outer.nodes[0].src_location, location);

    let drops = Arc::new(AtomicUsize::new(0));
    let tree = ErTree::new(ReportOutputErr, [Tracked::new(85, Arc::clone(&drops))]);
    #[cfg(feature = "src_locations")]
    let location = tree.src_location;
    let child = tree.er_find::<Tracked>().unwrap() as *const Tracked;
    let result: Result<(), ErReport<ReportOutputErr>> = Err(tree.into_er_report().single_line());
    let top = result.er_tree().er_top().unwrap_err();
    assert_eq!(top.layout, Layout::Multiline);

    let result: Result<(), ErTop<ReportOutputErr>> = Err(top.single_line());
    let outer = result.er_tree().er(HandlerErr::new).unwrap_err();

    assert!(outer.er_contains::<ReportOutputErr>());
    assert!(!outer.er_contains::<ErReport<ReportOutputErr>>());
    assert!(ptr::eq(child, outer.er_find::<Tracked>().unwrap()));
    #[cfg(feature = "src_locations")]
    assert_eq!(outer.nodes[0].src_location, location);
    assert_eq!(drops.load(Ordering::Relaxed), 0);
    drop(outer);
    assert_eq!(drops.load(Ordering::Relaxed), 1);
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
