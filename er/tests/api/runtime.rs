use er::*;
use std::io::Error;
#[cfg(feature = "macros")]
use std::io::ErrorKind;

pub fn read_port(input: &str) -> Er<u16, Error> {
    input.parse().er(|| Error::other("port"))
}

#[test]
pub fn without_derives() {
    assert_eq!(read_port("85").er_report().unwrap(), 85);

    let results = [Err::<(), _>(Error::other("mode"))];
    let error: ErTree<Error> = aggregate::collect(|| Error::other("config"), results).unwrap_err();
    assert_eq!(error.nodes.len(), 1);

    #[cfg(feature = "macros")]
    {
        let error: ErTree<Error> = er_all!(
            || Error::other("config"),
            [
                read_port("fakenumber"),
                None::<()>.er::<Error>(|| Error::new(ErrorKind::NotFound, "mode")),
            ]
        )
        .err()
        .unwrap();

        assert_eq!(error.nodes.len(), 2);
        assert!(error.er_find::<std::num::ParseIntError>().is_some());
        assert_eq!(error.er_top().to_string(), "config");
        assert!(error.er_report().to_string().contains("invalid digit"));
    }

    let root = Error::other("fresh").er();
    assert!(root.nodes.is_empty());
}

pub struct ManualWrap<E>(pub ErTree<E>);
impl<E> IntoErTree for ManualWrap<E> {
    type Error = E;

    fn into_er_tree(self) -> ErTree<E> {
        self.0
    }
}

#[test]
pub fn manual_wrap() {
    let text = String::from("85");
    let tree = ErTree {
        top: text.as_str(),
        nodes: Vec::new(),
        #[cfg(feature = "stack_traces")]
        stack_traces: Vec::new(),
        #[cfg(feature = "src_locations")]
        src_location: std::panic::Location::caller(),
    };

    let report = ManualWrap(tree).into_er_report();
    let top = ManualWrap(report.tree).into_er_top();

    assert_eq!(top.to_string(), "85");

    let result: Result<(), ErTop<_>> = Err(top);
    let report = result.er_tree().er_report().unwrap_err();
    let tree = report.into_er_tree();
    assert!(core::ptr::eq(tree.top, text.as_str()));
}

#[cfg(feature = "test")]
#[test]
pub fn test_helpers() -> ErTest {
    let port = read_port("85").er(())?;
    assert_eq!(port, 85);

    Ok(())
}
