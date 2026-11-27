use core::{error::Error, fmt};
use er::*;
use std::io;

#[derive(Debug)]
pub struct DriverErr(pub io::Error);
impl fmt::Display for DriverErr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("driver rejected the request")
    }
}
impl Error for DriverErr {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.0)
    }
}

#[derive(Er)]
pub struct StageErr;

#[derive(Er)]
pub struct OuterErr;
#[test]
pub fn ingestion() {
    let cause = io::Error::new(io::ErrorKind::PermissionDenied, "no access");
    let result: Result<(), Box<dyn Error + Send + Sync>> = Err(Box::new(DriverErr(cause)));
    let tree = result.er(StageErr::new).er(OuterErr::new).unwrap_err();
    assert!(tree.er_contains::<DriverErr>());

    let boxed: ErBoxError = Box::new(io::Error::from(io::ErrorKind::PermissionDenied));
    assert!(boxed.er(StageErr::new).er_contains::<io::Error>());

    let boxed: ErBoxError = Box::new(io::Error::from(io::ErrorKind::NotFound));
    let tree = boxed.er_with::<io::Error, _>(|err| {
        io::Error::from(err.downcast_ref::<io::Error>().unwrap().kind())
    });
    assert_eq!(tree.top.kind(), io::ErrorKind::NotFound);

    let result: Result<(), String> = Err(String::from("plain message"));
    let tree = result.er(StageErr::new).unwrap_err();
    assert!(tree.er_report().to_string().contains("plain message"));

    let result: Result<(), &'static str> = Err("static message");
    let tree = result.er(StageErr::new).unwrap_err();
    assert!(tree.er_report().to_string().contains("static message"));
}

#[test]
pub fn with_report() {
    let boxed: ErBoxError = Box::new(io::Error::from(io::ErrorKind::PermissionDenied));
    let result: Result<(), _> = Err((boxed, String::from("driver report")));
    let tree = result.er(StageErr::new).unwrap_err();

    assert!(tree.er_contains::<io::Error>());
    assert!(tree.er_report().contains("driver report"));
}

#[test]
pub fn saved_plain_error() {
    let error = io::Error::from(io::ErrorKind::PermissionDenied);
    let (error, report) = Err::<(), _>(error)
        .er_save("request", |_| "opening preferences")
        .er_split()
        .unwrap_err();

    assert_eq!(error.kind(), io::ErrorKind::PermissionDenied);
    assert!(report.contains("opening preferences"));
}

#[test]
pub fn blank_report() {
    let cause = DriverErr(io::Error::other("device busy"));
    let expected = cause.er_report();
    let result = Err::<(), _>((cause, String::from(" \t\n")));

    assert_eq!(result.as_ref().er_report().unwrap_err(), expected);
    assert_eq!(result.er_report().unwrap_err(), expected);

    let result = Err::<(), _>((StageErr, Some(String::new())));
    assert!(result.er(OuterErr::new).unwrap_err().saved.is_empty());
}

#[test]
pub fn pair_presentation() {
    fn failure() -> Result<(), (ErBoxError, String)> {
        Err((
            Box::new(DriverErr(io::Error::other("no access"))),
            "driver report".into(),
        ))
    }

    let (error, _) = failure().unwrap_err();
    let raw = "driver report\r\n`- request [saved]\r\n\r\n";
    let pair = Err::<(), _>((error, raw.to_owned()));
    assert_eq!(pair.as_ref().er_report().unwrap_err(), raw);
    assert_eq!(pair.er_report().unwrap_err(), raw);

    let saved = failure()
        .er_save("request", |_| "opening preferences")
        .er_save("request", |_| "opening preferences")
        .er_save("retry", |_| "could not retry");

    let borrowed = saved.as_ref().er_report().unwrap_err();
    let (cause, report) = saved.er_split().unwrap_err();
    assert!(cause.is::<DriverErr>());
    assert_eq!(borrowed, report);
    assert_eq!(
        report,
        "driver report\n\n---- Saved: request ----\nopening preferences\n\n---- Saved: retry ----\ncould not retry"
    );

    let failed = failure();
    let snapshot = failed.as_ref().er_snapshot().unwrap_err();
    assert_eq!(snapshot.er_entries().nth(1).unwrap().message, "no access");

    #[cfg(target_has_atomic = "ptr")]
    {
        let shared = failure().er_share().unwrap_err();
        let borrowed: ErTree<StageErr> = (&shared).er_with_tree(|_| ());
        let tree: ErTree<StageErr> = shared.clone().er_with_tree(|_| ());

        assert!(borrowed.er_contains_shared(&shared));
        assert!(tree.er_contains_shared(&shared));
        assert!(tree.er_contains::<DriverErr>());
        assert!(tree.er_report().contains("driver report"));

        let shared = failure().er_share().unwrap_err();
        let owned = std::sync::Arc::into_inner(shared.tree).unwrap();
        let (error, report) = Err::<(), _>(owned).er_split().unwrap_err();

        assert!(error.is::<DriverErr>());
        assert!(report.contains("driver report"));
    }
}

#[test]
pub fn erased_nodes() {
    let node = ErTree::from(StageErr).into_er_node();
    let cause = io::Error::new(io::ErrorKind::PermissionDenied, "no access");
    let boxed: ErBoxError = Box::new(DriverErr(cause));
    let tree = ErTree::new(OuterErr, [node, boxed.into_er_node()]);

    assert!(tree.er_contains::<StageErr>());
    assert!(tree.er_contains::<DriverErr>());
}

#[test]
pub fn presentations() {
    macro_rules! check {
        ($convert:expr) => {
            let sub = ErTree::new(StageErr, [io::Error::other("source")]);
            let boxed = Box::new(($convert)(sub));
            let tree: ErTree<OuterErr> = Err::<(), _>(boxed).er(()).unwrap_err();
            assert!(tree.er_contains::<io::Error>());

            let boxed = Box::new(($convert)(tree));
            let tree: ErTree<OuterErr> = er_all!((), [boxed]).unwrap_err();
            assert!(tree.er_contains::<io::Error>());
        };
    }

    check!(|tree| tree);
    check!(|tree| {
        let ErTree {
            top,
            nodes,
            saved,
            src_location,
        } = tree;

        ErTree {
            top: Box::new(top) as ErBoxError,
            nodes,
            saved,
            src_location,
        }
    });
}
