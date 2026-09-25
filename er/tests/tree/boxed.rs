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
    let tree = result.er::<StageErr>(()).er::<OuterErr>(()).unwrap_err();
    let driver = tree.er_find::<DriverErr>().unwrap();
    let source = tree.er_find::<io::Error>().unwrap();

    assert!(core::ptr::eq(source, &driver.0));
    assert_eq!(source.kind(), io::ErrorKind::PermissionDenied);
    assert!(tree.er_contains::<OuterErr>());
    assert!(tree.er_contains::<StageErr>());
    assert!(!tree.er_contains::<fmt::Error>());
    assert_eq!(tree.er_sources().count(), 0);
    assert_eq!(
        tree.er_descendants()
            .map(|node| node.error.to_string())
            .collect::<Vec<_>>(),
        ["StageErr", "driver rejected the request"]
    );

    let report = tree.er_report().to_string();

    for text in [
        "OuterErr",
        "StageErr",
        "driver rejected the request",
        "no access",
    ] {
        assert!(report.contains(text), "{report}");
    }

    assert!(!report.contains("Box<"));

    let result: Result<(), String> = Err(String::from("plain message"));
    let tree = result.er::<StageErr>(()).unwrap_err();

    assert!(tree.er_report().to_string().contains("plain message"));

    let result: Result<(), &'static str> = Err("static message");
    let tree = result.er::<StageErr>(()).unwrap_err();

    assert!(tree.er_report().to_string().contains("static message"));
}

#[test]
pub fn erased_nodes() {
    let inner = StageErr.er();
    #[cfg(feature = "src_locations")]
    let src = inner.src_location;
    let node = inner.into_er_part();
    #[cfg(feature = "src_locations")]
    assert_eq!(node.node.src_location, src);

    let cause = io::Error::new(io::ErrorKind::PermissionDenied, "no access");
    let boxed: BoxError = Box::new(DriverErr(cause));
    let tree = ErTree::new(OuterErr, [node, boxed.into_er_part()]);

    assert!(tree.er_contains::<StageErr>());
    assert!(tree.er_contains::<DriverErr>());
    assert_eq!(tree.er_descendants().count(), 2);
    #[cfg(feature = "src_locations")]
    assert_eq!(tree.nodes[0].src_location, src);
}
