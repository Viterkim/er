use er::*;
use std::io::Error;
#[cfg(feature = "macros")]
use std::io::ErrorKind;

pub fn read_port(input: &str) -> ErResult<u16, Error> {
    input.parse().er(|| Error::other("port"))
}

#[test]
pub fn without_derives() {
    assert_eq!(read_port("85").er_report().unwrap(), 85);
    read_port("85").unwrap_report();
    read_port("85").expect_report("bad port");

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
    }

    struct SavedText(String);
    impl From<((String, Error),)> for SavedText {
        fn from(((text, _),): ((String, Error),)) -> Self {
            Self(text)
        }
    }

    let saved: SavedText = read_port("bad")
        .er_report()
        .unwrap_err()
        .single_line()
        .er_into(|report| {
            let text = report.er_report_string();
            (text, report.tree.top)
        });
    assert!(saved.0.contains("invalid digit"));

    let _: Result<u16, SavedText> = read_port("85").er_into(|_| -> (String, Error) {
        panic!("boundary constructor ran for a successful result");
    });

    let error = Error::other("source").er::<Error>(|| Error::other("context"));
    assert_eq!(error.nodes[0].error.to_string(), "source");
}

#[test]
#[should_panic(expected = "invalid digit found in string")]
pub fn unwrap_report() {
    read_port("fakenumber").unwrap_report();
}

#[test]
#[should_panic(expected = "bad port: port")]
pub fn expect_report() {
    read_port("fakenumber").expect_report("bad port");
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
        #[cfg(feature = "stack_traces")]
        stack_traces: Vec::new(),
        #[cfg(feature = "src_locations")]
        src_location: std::panic::Location::caller(),
    };

    let report = ManualWrap(tree).into_er_report();
    let top = ManualWrap(report.tree).into_er_top();

    let result: Result<(), ErTop<_>> = Err(top);
    let tree = result.er_tree().er_report().unwrap_err().into_er_tree();
    assert_eq!(tree.top, "85");
}
