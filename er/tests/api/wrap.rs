use core::num::ParseIntError;
use er::*;

#[derive(Er)]
pub struct InnerErr;

#[derive(Er)]
#[er(wrap)]
pub struct HandlerErr;

pub fn parse_wrapped(input: &str) -> Result<u16, HandlerErrWrap> {
    let port: u16 = input.parse().er(())?;
    Ok(port)
}

#[test]
pub fn conversions() {
    let error = ErTree::new(HandlerErr, [ErTree::new(InnerErr, [InnerErr])]);

    fn propagate(result: Result<(), HandlerErrWrap>) -> ErResult<(), HandlerErr> {
        result?;
        Ok(())
    }

    let recovered = propagate(Err(HandlerErrWrap::from(error))).unwrap_err();
    assert_eq!(recovered.er_find_all::<InnerErr>().count(), 2);
}

#[test]
pub fn wrapped_results() {
    assert_eq!(parse_wrapped("85").er_report().unwrap(), 85);

    let error = parse_wrapped("bad").unwrap_err();
    assert!(error.er_contains::<ParseIntError>());
}

#[test]
pub fn saved_wrap() {
    let result = parse_wrapped("bad").er_save("request", |_| "opening preferences");
    let (_, report): (HandlerErr, _) = result.er_split().unwrap_err();
    assert!(report.contains("opening preferences"));
}

#[test]
pub fn fresh_conversions() {
    let wrapped: HandlerErrWrap = HandlerErr::new().into();
    assert!(wrapped.tree.nodes.is_empty());

    fn from_result() -> Result<(), HandlerErrWrap> {
        Err::<(), _>(HandlerErr::new())?;
        Ok(())
    }

    assert!(from_result().unwrap_err().tree.nodes.is_empty());
}

#[derive(Er)]
#[er(wrap(output = top))]
pub struct TopOutputErr;
#[test]
pub fn top_output() {
    let wrapped = TopOutputErrWrap::from(ErTree::new(TopOutputErr, [InnerErr]));
    assert_eq!(wrapped.to_string(), "TopOutputErr");

    let result = Err::<(), _>(Box::new(wrapped));
    let tree = result.er(HandlerErr::new).unwrap_err();
    assert!(tree.er_contains::<InnerErr>());
}

#[derive(Er)]
#[er(wrap(output = report))]
pub struct ReportOutputErr;
#[test]
pub fn report_output() {
    let wrapped = ReportOutputErrWrap::from(ErTree::new(ReportOutputErr, [InnerErr]));
    assert!(wrapped.to_string().contains("InnerErr"));
}

#[derive(Er)]
#[er(wrap(output = report, std_error))]
pub struct StandardErr<T>(pub T);

#[derive(Er)]
pub struct RequestErr {
    pub input: String,
}

#[test]
pub fn saved_std_error() {
    let failed = || {
        Err::<(), _>(StandardErrWrap::from(ErTree::new(
            StandardErr(85),
            [InnerErr],
        )))
    };
    let (_wrapped, _): (StandardErrWrap<i32>, _) = failed()
        .er_save("request", |_| "opening preferences")
        .er_split()
        .unwrap_err();

    let tree = failed()
        .er_save_wrap("request", |_| "opening preferences")
        .unwrap_err();
    assert!(tree.er_contains::<InnerErr>());
    let (_inner, report): (StandardErr<i32>, _) = tree.er_split();
    assert!(report.contains("opening preferences"));
}

#[test]
pub fn std_error() {
    let tree = ErTree::new(StandardErr::new(85u8), [InnerErr]);
    let expected = tree.er_report().to_string();
    let wrapped = StandardErrWrap::from(tree);
    assert_eq!(wrapped.to_string(), expected);

    let tree: ErTree<RequestErr> = Err::<(), _>(Box::new(wrapped))
        .er_with_wrap(|err| err.0.to_string())
        .unwrap_err();
    assert_eq!(tree.top.input, "85");
    assert!(tree.er_contains::<InnerErr>());

    let wrapped = || StandardErrWrap::from(ErTree::new(StandardErr::new(85u8), [InnerErr]));
    let failed = (|| -> ErTest { er_bail!(wrapped()) })().unwrap_err();
    assert!(failed.er_contains::<InnerErr>());

    let saved = Err::<(), _>(wrapped()).er_save("request", |_| "sent");
    let combined: ErResult<(), HandlerErr> = er_all!((), [saved]);
    assert!(combined.unwrap_err().er_contains::<InnerErr>());

    let shared: ErSharedTree<StandardErr<u8>> = wrapped().er_share();
    assert!(shared.er_contains::<InnerErr>());

    let outer = Err::<(), _>(wrapped()).er(HandlerErr::new).unwrap_err();
    assert!(outer.er_contains::<StandardErrWrap<u8>>());

    let _: ErResult<(), RequestErr> = Err::<(), _>(wrapped()).er_wrap(|_| "port");
    let tree: ErTree<HandlerErr> = Box::new(wrapped()).er_wrap(HandlerErr::new);
    assert!(tree.er_contains::<InnerErr>());
    let _ = Err::<(), _>(wrapped()).er_snapshot();

    let mut called = false;
    let _ = Ok::<_, StandardErrWrap<u8>>(7).er_with_wrap::<HandlerErr, _>(|_| called = true);
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
        GenericWrap::from(tree).into_er_tree()
    }

    let error = ErTree::from(GenericErr { value: 7u8 });
    let wrapped = GenericWrap::from(roundtrip(error));

    let result: Result<(), GenericWrap<u8>> = Err(wrapped);
    let tree = result
        .er_with::<RequestErr, _>(|err| err.value.to_string())
        .unwrap_err();
    assert_eq!(tree.top.input, "7");

    let wrapped = GenericWrap::from(ErTree::new(GenericErr::new(8u8), [InnerErr]));
    let tree = wrapped.er_with_tree::<RequestErr, _>(|tree| tree.top.value.to_string());
    assert_eq!(tree.top.input, "8");
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
        saved: ErSavedList::default(),
        #[cfg(feature = "src_locations")]
        src_location: std::panic::Location::caller(),
    };
    let wrapped = BorrowedErrWrap::from(tree);

    let tree: ErTree<BorrowedErr<'_>> = wrapped.into();
    assert_eq!(tree.top.text, text);
}
