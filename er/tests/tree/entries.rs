use er::*;
use std::io::Error as IoError;
use std::marker::PhantomPinned;
use std::{error::Error, fmt};

#[derive(Er)]
pub struct Leaf(pub u8);

#[derive(Debug)]
pub struct Native(pub Leaf);
impl fmt::Display for Native {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("native")
    }
}
impl Error for Native {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.0)
    }
}

#[test]
pub fn lookup() {
    let group = ErTree::new(Native(Leaf(99)), [Leaf(1), Leaf(2)]);
    let read: ErResult<(), Leaf> = Err("bad".parse::<u8>().unwrap_err()).er(|_| 3);

    let tree: ErTree<Leaf> =
        er_all!(|_| 0, [Err::<(), _>(group), Ok::<u8, Leaf>(7), read]).unwrap_err();

    let leaf =
        |error: Option<&(dyn Error + 'static)>| error.unwrap().downcast_ref::<Leaf>().unwrap().0;
    for (index, path, value) in [
        (0, &[][..], 0),
        (2, &[0, 0][..], 1),
        (3, &[0, 1][..], 2),
        (4, &[1][..], 3),
    ] {
        let by_index = tree.er_top().er_at_index(ErErrorIndex(index));
        let by_path = tree.er_report().er_at_path(path);
        assert_eq!((leaf(by_index), leaf(by_path)), (value, value));
    }
    assert!(tree.er_at_index(ErErrorIndex(6)).is_none());
    assert!(tree.er_at_path(&[0, 2]).is_none());
}

#[test]
pub fn walk() {
    let tree = ErTree::new(
        Native(Leaf(0)),
        [
            ErTree::new(Native(Leaf(1)), [ErTree::from(Leaf(2))]).into_er_part(),
            ErTree::from(Leaf(3)).into_er_part(),
        ],
    );
    let report = tree.into_er_report();
    let _: ErFindAll<'_, Leaf> = report.as_ref().er_find_all();

    let entries: Vec<_> = report.as_ref().er_entries().collect();
    let mut visited = Vec::new();
    report
        .er_top()
        .er_for_each_entry(|entry| visited.push(entry.index));

    assert!(visited.iter().eq(entries.iter().map(|entry| &entry.index)));

    use ErEntryKind::{Node, Root, Source};

    assert_eq!(
        entries
            .iter()
            .map(|e| (e.index, e.kind, e.parent, e.depth, e.is_last))
            .collect::<Vec<_>>(),
        [
            (0, Root, None, 0, true),
            (1, Source, Some(0), 1, false),
            (2, Node, Some(0), 1, false),
            (3, Source, Some(2), 2, false),
            (4, Node, Some(2), 2, true),
            (5, Node, Some(0), 1, true),
        ]
    );

    let descendants: Vec<_> = report
        .er_descendants()
        .map(|node| node.error.to_string())
        .collect();
    assert_eq!(descendants, ["native", "Leaf(2)", "Leaf(3)"]);
}

#[test]
pub fn clone() {
    let tree = ErTree::new(Native(Leaf(0)), [Native(Leaf(1)), Native(Leaf(2))]);

    let mut entries = tree.er_entries();
    entries.next();
    assert_eq!(
        entries.clone().map(|entry| entry.index).collect::<Vec<_>>(),
        entries.map(|entry| entry.index).collect::<Vec<_>>()
    );

    let mut nodes = tree.er_descendants();
    nodes.next();
    assert_eq!(nodes.clone().count(), nodes.count());

    let mut sources = ErSources::new(Some(&tree.top));
    sources.next();
    assert_eq!(sources.clone().count(), sources.count());
}

#[test]
pub fn hidden_payloads() {
    let tree = ErTree::from(IoError::other(Leaf(7)));
    assert!(tree.er_find::<Leaf>().is_none());

    let tree = ErTree::from(Box::new(Leaf(8)));
    assert!(tree.er_find::<Leaf>().is_none());
}

#[test]
pub fn lazy_matches() {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    #[derive(Debug)]
    pub struct SharedSource {
        pub calls: Arc<AtomicUsize>,
    }
    impl fmt::Display for SharedSource {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("shared source")
        }
    }
    impl Error for SharedSource {
        fn source(&self) -> Option<&(dyn Error + 'static)> {
            static SOURCE: Leaf = Leaf(7);
            self.calls.fetch_add(1, Ordering::Relaxed);
            Some(&SOURCE)
        }
    }

    let calls = Arc::new(AtomicUsize::new(0));
    let tree = ErTree::new(
        SharedSource {
            calls: calls.clone(),
        },
        [SharedSource {
            calls: calls.clone(),
        }],
    );

    let mut matches = tree.er_find_all::<Leaf>();
    assert_eq!(calls.load(Ordering::Relaxed), 0);

    matches.next().unwrap();
    assert_eq!(calls.load(Ordering::Relaxed), 1);
}

#[test]
pub fn nested_matches() {
    let tree = ErTree::new(
        Leaf(0),
        [
            ErTree::new(
                Leaf(1),
                [ErTree::new(Leaf(2), [Leaf(3)]), ErTree::from(Leaf(4))],
            ),
            ErTree::from(Leaf(5)),
        ],
    );
    let matches = tree
        .er_find_all::<Leaf>()
        .map(|leaf| leaf.0)
        .collect::<Vec<_>>();

    assert_eq!(matches, [0, 1, 2, 3, 4, 5]);
}

#[derive(Debug)]
pub struct Pinned(pub PhantomPinned);
impl fmt::Display for Pinned {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("pinned")
    }
}
impl Error for Pinned {}

#[test]
pub fn find_all_is_unpin() {
    pub fn needs_unpin(_: impl Unpin) {}

    let tree = ErTree::from(Pinned(PhantomPinned));
    needs_unpin(tree.er_find_all::<Pinned>());
}

#[test]
#[cfg(feature = "src_locations")]
pub fn src_locations() {
    let line = line!() + 1;
    let direct = ErTree::from(Leaf(0));
    let src: ErSrcLocation = direct.src_location;

    assert_eq!((src.file(), src.line()), (file!(), line));

    let plain = Leaf(1);
    let existing = direct;
    let line = line!() + 4;
    let tree: ErTree<Leaf> = er_all!(
        |_| 2,
        [
            plain,
            "bad".parse::<u16>().unwrap_err(),
            "not a valid boolean option value".parse::<bool>(),
            existing,
        ]
    )
    .unwrap_err();

    assert_eq!(tree.nodes[0].src_location.line(), line);
    assert_eq!(tree.nodes[2].src_location.line(), line + 2);
    assert_eq!(tree.nodes[3].src_location, src);

    let missing: Option<()> = None;
    let line = line!() + 1;
    let tree = missing.er::<Leaf>(|_| 3).unwrap_err();

    assert_eq!(tree.src_location.line(), line);

    let line = line!() + 1;
    let tree = ErTree::new(Leaf(5), [Leaf(6)]);

    assert_eq!(tree.nodes[0].src_location.line(), line);

    let boxed: Box<dyn Error + Send + Sync> = Box::new(Leaf(7));
    let result: Result<(), BoxError> = Err(boxed);
    let line = line!() + 1;
    let tree = result.er::<Leaf>(|_| 8).unwrap_err();

    assert_eq!(tree.nodes[0].src_location.line(), line);

    let boxed: BoxError = Box::new(Leaf(9));
    let line = line!() + 1;
    let node = boxed.into_er_part();

    assert_eq!(node.node.src_location.line(), line);
}
