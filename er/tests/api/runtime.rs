use er::*;
use std::io::Error;
#[cfg(feature = "macros")]
use std::io::ErrorKind;
use std::task::Poll;

pub fn read_port(input: &str) -> ErResult<u16, Error> {
    input.parse().er(|| Error::other("port"))
}

#[test]
pub fn without_derives() {
    assert_eq!(read_port("85").er_report().unwrap(), 85);
    read_port("85").er_unwrap();
    read_port("85").er_expect("bad port");

    let results = [Err::<(), _>(Error::other("mode"))];
    let error = results
        .into_iter()
        .er_collect_all::<(), Error>(|| Error::other("config"))
        .unwrap_err();
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
    }

    struct SavedText(String);

    let saved = read_port("bad")
        .er_split_with(|(_, report)| SavedText(report))
        .unwrap_err();
    assert!(saved.0.contains("invalid digit"));

    let _: Result<u16, SavedText> = read_port("85").er_split_with(|_| -> SavedText {
        panic!("boundary constructor ran for a successful result");
    });

    let error = Error::other("source").er::<Error>(|| Error::other("context"));
    assert_eq!(error.nodes[0].error.to_string(), "source");
}

#[test]
#[should_panic(expected = "invalid digit found in string")]
pub fn er_unwrap() {
    read_port("fakenumber").er_unwrap();
}

#[test]
#[should_panic(expected = "bad port: port")]
pub fn er_expect() {
    read_port("fakenumber").er_expect("bad port");
}

#[test]
pub fn poll_result() {
    let input: Poll<Result<(), _>> = Poll::Ready(Err(std::fmt::Error));
    let Poll::Ready(Err(tree)) = input.er(|| Error::other("write")) else {
        panic!("expected a write error");
    };

    assert!(tree.er_find::<std::fmt::Error>().is_some());
    #[cfg(feature = "src_locations")]
    assert_eq!(tree.src_location.unwrap().file(), file!());
}

#[test]
pub fn stream_end() {
    let polled: Poll<Option<Result<u8, Error>>> = Poll::Ready(None);
    let result = polled.er(|| -> Error { panic!("stream ended cleanly") });
    assert!(matches!(result, Poll::Ready(None)));
}

#[test]
pub fn collections() {
    let input = || {
        ["1", "bad", "2", "also bad"]
            .into_iter()
            .map(str::parse::<u8>)
    };
    let mut values = input();
    let result = values
        .by_ref()
        .er_collect::<Vec<_>, Error>(|| Error::other("parse"));
    assert_eq!(result.unwrap_err().nodes.len(), 1);
    assert_eq!(values.next().unwrap().unwrap(), 2);

    let result = input().er_collect_all::<Vec<_>, Error>(|| Error::other("parse"));
    assert_eq!(result.unwrap_err().nodes.len(), 2);
    let result = [Ok::<_, Error>(1), Ok(2)]
        .into_iter()
        .er_collect_all::<Vec<_>, Error>(|| Error::other("parse"));
    assert_eq!(result.er_report().unwrap(), [1, 2]);
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
        saved: ErSavedList::default(),
        src_location: None,
    };

    let result: Result<(), ManualWrap<_>> = Err(ManualWrap(tree));
    let tree = result.er_tree().unwrap_err();
    assert_eq!(tree.top, "85");
}
