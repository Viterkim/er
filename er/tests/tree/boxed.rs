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

    let boxed: BoxError = Box::new(io::Error::from(io::ErrorKind::PermissionDenied));
    assert!(boxed.er(StageErr::new).er_contains::<io::Error>());

    let boxed: BoxError = Box::new(io::Error::from(io::ErrorKind::NotFound));
    let tree = boxed.er_with::<io::Error, _>(|err| {
        er_built(io::Error::from(
            err.downcast_ref::<io::Error>().unwrap().kind(),
        ))
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
pub fn erased_nodes() {
    let node = ErTree::from(StageErr).into_er_part();
    let cause = io::Error::new(io::ErrorKind::PermissionDenied, "no access");
    let boxed: BoxError = Box::new(DriverErr(cause));
    let tree = ErTree::new(OuterErr, [node, boxed.into_er_part()]);

    assert!(tree.er_contains::<StageErr>());
    assert!(tree.er_contains::<DriverErr>());
}

#[test]
pub fn presentations() {
    macro_rules! check {
        ($convert:expr) => {
            let child = ErTree::new(StageErr, [io::Error::other("source")]);
            let boxed = Box::new(($convert)(child));
            let tree: ErTree<OuterErr> = Err::<(), _>(boxed).er(()).unwrap_err();
            assert!(tree.er_contains::<io::Error>());

            let boxed = Box::new(($convert)(tree));
            let tree: ErTree<OuterErr> = er_all!((), [boxed]).unwrap_err();
            assert!(tree.er_contains::<io::Error>());
        };
    }

    check!(|tree| tree);
    check!(ErTree::into_er_top);
    check!(ErTree::into_er_report);
}
