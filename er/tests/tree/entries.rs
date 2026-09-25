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
    let read: ErResult<(), Leaf> = Err("bad".parse::<u8>().unwrap_err()).er(|| Leaf(3));
    #[cfg(feature = "stack_traces")]
    let read = read.er_trace();
    let tree: ErTree<Leaf> =
        er_all!(|| Leaf(0), [Err::<(), _>(group), Ok::<u8, Leaf>(7), read]).unwrap_err();

    assert!(tree.er_at_index(ErErrorIndex(1)).unwrap().is::<Native>());
    for (index, path, value) in [
        (0, &[][..], 0),
        (2, &[0, 0][..], 1),
        (3, &[0, 1][..], 2),
        (4, &[1][..], 3),
    ] {
        let by_index = tree.er_at_index(ErErrorIndex(index)).unwrap();
        let by_path = tree.er_at_path(path).unwrap();
        assert!(std::ptr::eq(by_index, by_path));
        assert_eq!(by_index.downcast_ref::<Leaf>().unwrap().0, value);
    }
    assert!(
        tree.er_at_index(ErErrorIndex(5))
            .unwrap()
            .is::<std::num::ParseIntError>()
    );
    assert!(tree.er_at_index(ErErrorIndex(6)).is_none());
    assert!(tree.er_at_path(&[0, 2]).is_none());
    assert!(tree.er_at_path(&[1, 1]).is_none());
    assert!(tree.er_find_all::<Leaf>().any(|leaf| leaf.0 == 99));

    #[cfg(feature = "stack_traces")]
    {
        assert_eq!(tree.stack_traces.len(), 1);
        assert_eq!(tree.stack_traces[0].error_index, ErErrorIndex(4));
        assert_eq!(
            tree.er_at_index(tree.stack_traces[0].error_index)
                .unwrap()
                .downcast_ref::<Leaf>()
                .unwrap()
                .0,
            3
        );
    }
    let tree = tree.er(|| Leaf(8));
    assert_eq!(
        tree.er_at_index(ErErrorIndex(5))
            .unwrap()
            .downcast_ref::<Leaf>()
            .unwrap()
            .0,
        3
    );
    #[cfg(feature = "stack_traces")]
    {
        assert_eq!(tree.stack_traces[0].error_index, ErErrorIndex(5));
        assert_eq!(
            tree.er_at_index(tree.stack_traces[0].error_index)
                .unwrap()
                .downcast_ref::<Leaf>()
                .unwrap()
                .0,
            3
        );
    }
}

#[test]
pub fn walk() {
    let tree = ErTree::new(
        Native(Leaf(0)),
        [
            ErTree::new(Native(Leaf(1)), [Leaf(2).er()]).into_er_part(),
            Leaf(3).er().into_er_part(),
        ],
    );
    let report = tree.into_er_report();

    let matches = report.as_ref().er_find_all::<Leaf>();
    assert_eq!(matches.map(|leaf| leaf.0).collect::<Vec<_>>(), [0, 1, 2, 3]);
    assert!(std::ptr::eq(
        report.er_find_all::<Native>().next().unwrap(),
        &report.tree.top,
    ));
    assert!(std::ptr::eq(
        report.er_find_all::<Leaf>().next().unwrap(),
        report.er_find::<Leaf>().unwrap(),
    ));
    assert!(report.er_find_all::<IoError>().next().is_none());

    let matches = report.tree.er_top().er_find_all::<Leaf>();
    assert_eq!(matches.count(), 4);
    assert_eq!(
        report.tree.nodes[0]
            .er_find_all::<Leaf>()
            .map(|leaf| leaf.0)
            .collect::<Vec<_>>(),
        [1, 2],
    );

    let entries: Vec<_> = report.as_ref().er_entries().collect();
    let mut visited = Vec::new();
    report.er_for_each_entry(|entry| visited.push(entry));

    assert_eq!(visited.len(), entries.len());

    for (callback, entry) in visited.iter().zip(&entries) {
        assert!(std::ptr::eq(callback.error, entry.error));
        assert_eq!(callback.index, entry.index);
    }

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
    assert_eq!(
        entries
            .iter()
            .map(|e| e.error.to_string())
            .collect::<Vec<_>>(),
        [
            "native", "Leaf(0)", "native", "Leaf(1)", "Leaf(2)", "Leaf(3)"
        ]
    );

    #[cfg(feature = "src_locations")]
    for entry in entries {
        assert_eq!(entry.src_location.is_some(), entry.kind != Source);
    }

    let descendants: Vec<_> = report
        .er_descendants()
        .map(|node| node.error.to_string())
        .collect();
    assert_eq!(descendants, ["native", "Leaf(2)", "Leaf(3)"]);
    assert_eq!(report.tree.nodes[0].er_descendants().count(), 1);

    let mut nodes = ErNodes::new(&report.tree.nodes);
    assert_eq!(nodes.pending.len(), 2);
    nodes.pending.clear();

    assert!(nodes.next().is_none());

    #[cfg(not(feature = "src_locations"))]
    {
        assert_eq!(
            report.to_string(),
            "native\n|- Leaf(0)\n|- native\n|  |- Leaf(1)\n|  `- Leaf(2)\n`- Leaf(3)"
        );
        assert_eq!(
            report.as_ref().single_line().to_string(),
            "native [Leaf(0) | native [Leaf(1) | Leaf(2)] | Leaf(3)]"
        );
    }
}

#[test]
pub fn clone() {
    let tree = ErTree::new(Native(Leaf(0)), [Native(Leaf(1)), Native(Leaf(2))]);

    let mut entries = tree.er_entries();
    assert_eq!(entries.next().map(|entry| entry.index), Some(0));
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
pub fn root_and_source() {
    let tree = Native(Leaf(9)).er();
    assert_eq!(tree.er_descendants().count(), 0);
    assert_eq!(tree.er_report().er_entries().count(), 2);

    let top = Leaf(9).er().into_er_top();
    assert!(std::ptr::eq(
        top.er_find_all::<Leaf>().next().unwrap(),
        &top.tree.top
    ));
    assert_eq!(top.er_find_all::<Leaf>().count(), 1);
}

#[test]
pub fn hidden_payloads() {
    let tree = IoError::other(Leaf(7)).er();
    assert!(tree.er_find::<Leaf>().is_none());
    assert!(tree.er_find_all::<Leaf>().next().is_none());
    assert_eq!(tree.er_entries().count(), 1);

    let io = tree.er_find::<IoError>().unwrap();
    assert_eq!(io.get_ref().unwrap().downcast_ref::<Leaf>().unwrap().0, 7);

    let tree = Box::new(Leaf(8)).er();
    assert!(tree.er_find::<Leaf>().is_none());
    assert_eq!(tree.er_find::<Box<Leaf>>().unwrap().0, 8);
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

    assert!(tree.er_find_all::<SharedSource>().next().is_some());
    assert_eq!(calls.load(Ordering::Relaxed), 0);

    let mut matches = tree.er_find_all::<Leaf>();
    assert_eq!(calls.load(Ordering::Relaxed), 0);

    let first = matches.next().unwrap();
    assert_eq!(calls.load(Ordering::Relaxed), 1);

    let second = matches.next().unwrap();
    assert!(std::ptr::eq(first, second));
    assert_eq!(calls.load(Ordering::Relaxed), 2);

    assert!(matches.next().is_none());
    assert!(matches.next().is_none());
}

#[test]
pub fn nested_matches() {
    let tree = ErTree::new(
        Leaf(0),
        [
            ErTree::new(Leaf(1), [ErTree::new(Leaf(2), [Leaf(3)]), Leaf(4).er()]),
            Leaf(5).er(),
        ],
    );
    let entries = tree
        .er_entries()
        .filter_map(|entry| entry.error.downcast_ref::<Leaf>())
        .map(|leaf| leaf.0)
        .collect::<Vec<_>>();
    let matches = tree
        .er_find_all::<Leaf>()
        .map(|leaf| leaf.0)
        .collect::<Vec<_>>();

    assert_eq!(matches, entries);
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

    let tree = Pinned(PhantomPinned).er();
    needs_unpin(tree.er_find_all::<Pinned>());
}

#[test]
pub fn public_data() {
    pub fn inspect<E>(tree: ErTree<E>) -> E {
        let _: &E = &tree.top;
        let _ = tree.nodes;
        let _ = tree.er_descendants();
        let _: ErTopRef<'_, E> = (&tree).into();
        let _: ErReportRef<'_, E> = (&tree).into();
        let report: ErReport<E> = tree.into();
        let _: &E = &report.tree.top;
        let _ = &report.tree.nodes;
        let _ = report.as_ref().er_descendants();
        let tree: ErTree<E> = report.into();
        let top: ErTop<E> = tree.into();
        let tree: ErTree<E> = top.into();
        tree.top
    }

    assert_eq!(inspect(Leaf(1).er()).0, 1);
}

#[test]
#[cfg(feature = "src_locations")]
pub fn src_locations() {
    let line = line!() + 1;
    let direct = Leaf(0).er();
    let src: SrcLocation = direct.src_location;

    assert_eq!((src.file(), src.line()), (file!(), line));

    let plain = Leaf(1);
    let existing = direct;
    let line = line!() + 3;
    let first_line = line!() + 6;
    let second_line = line!() + 6;
    let tree: ErTree<Leaf> = er_all!(
        || Leaf(2),
        [
            plain,
            "bad".parse::<u16>().unwrap_err(),
            "not a valid boolean option value".parse::<bool>(),
            existing,
        ]
    )
    .unwrap_err();

    assert_eq!(tree.src_location.line(), line);
    assert_eq!(tree.nodes[0].src_location.line(), line + 3);
    assert_eq!(tree.nodes[1].src_location.line(), first_line);
    assert_eq!(tree.nodes[2].src_location.line(), second_line);
    assert_eq!(tree.nodes[3].src_location, src);

    let missing: Option<()> = None;
    let line = line!() + 1;
    let tree = missing.er::<Leaf>(|| Leaf(3)).unwrap_err();

    assert_eq!(tree.src_location.line(), line);

    let line = line!() + 1;
    let tree = ErTree::new(Leaf(4), [tree]);

    assert_eq!(tree.src_location.line(), line);

    let line = line!() + 1;
    let tree = ErTree::new(Leaf(5), [Leaf(6)]);

    assert_eq!(tree.nodes[0].src_location.line(), line);

    let boxed: Box<dyn Error + Send + Sync> = Box::new(Leaf(7));
    let result: Result<(), BoxError> = Err(boxed);
    let line = line!() + 1;
    let tree = result.er::<Leaf>(|| Leaf(8)).unwrap_err();

    assert_eq!(tree.src_location.line(), line);
    assert_eq!(tree.nodes[0].src_location.line(), line);

    let boxed: BoxError = Box::new(Leaf(9));
    let line = line!() + 1;
    let node = boxed.into_er_part();

    assert_eq!(node.node.src_location.line(), line);
}
