use core::{error::Error, fmt};
use er::*;
use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    thread::Builder,
};

#[derive(Debug)]
pub struct Counted {
    pub probes: Arc<AtomicUsize>,
}
impl fmt::Display for Counted {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("counted")
    }
}
impl Error for Counted {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.probes.fetch_add(1, Ordering::Relaxed);
        None
    }
}

pub fn linear_chain(depth: usize, probes: &Arc<AtomicUsize>) -> ErTree<Counted> {
    let mut error = Counted {
        probes: Arc::clone(probes),
    }
    .er();

    for _ in 0..depth {
        error = ErTree::new(
            Counted {
                probes: Arc::clone(probes),
            },
            [error],
        );
    }

    error
}

#[derive(Er)]
pub struct AbsentErr;
#[test]
pub fn visits_once() {
    let probes = Arc::new(AtomicUsize::new(0));
    let line = linear_chain(50, &probes);
    let branches = ErTree::new(
        Counted {
            probes: Arc::clone(&probes),
        },
        (0..8).map(|_| linear_chain(7, &probes)),
    );

    for (tree, count) in [(line, 51), (branches, 65)] {
        probes.store(0, Ordering::Relaxed);

        assert!(tree.er_find::<AbsentErr>().is_none());
        assert_eq!(probes.load(Ordering::Relaxed), count);
    }
}

pub struct NeverFormat;
impl fmt::Display for NeverFormat {
    fn fmt(&self, _formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        panic!("unexpected Display")
    }
}
impl fmt::Debug for NeverFormat {
    fn fmt(&self, _formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        panic!("unexpected Debug")
    }
}
impl Error for NeverFormat {}

#[test]
pub fn query_does_not_format() {
    let error = ErTree::new(NeverFormat, [NeverFormat.er()]);
    let error = ErTree::new(NeverFormat, [error, NeverFormat.er()]);

    assert!(error.er_find::<AbsentErr>().is_none());
    assert!(error.er_contains::<NeverFormat>());
    assert_eq!(error.er_descendants().count(), 3);
    assert_eq!(error.er_sources().count(), 0);
    assert_eq!(error.er_report().er_entries().count(), 4);

    let _top = error.er_top();
    let _report = error.er_report();

    let owned = error.into_er_report();
    assert!(owned.tree.er_contains::<NeverFormat>());

    let recovered = owned.tree;
    assert!(recovered.er_find::<NeverFormat>().is_some());
}

const CHAIN: usize = 50;

pub struct Deep;
impl fmt::Display for Deep {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("the buried cause")
    }
}
impl fmt::Debug for Deep {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("Deep")
    }
}
impl Error for Deep {}

pub struct Link {
    pub depth: usize,
    pub next: Option<Box<dyn Error + Send + Sync + 'static>>,
}
impl fmt::Display for Link {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "link {}", self.depth)
    }
}
impl fmt::Debug for Link {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("Link")
    }
}
impl Error for Link {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.next
            .as_deref()
            .map(|error| error as &(dyn Error + 'static))
    }
}

pub fn long_chain() -> Link {
    let mut next: Option<Box<dyn Error + Send + Sync + 'static>> = Some(Box::new(Deep));

    for depth in 0..CHAIN {
        next = Some(Box::new(Link { depth, next }));
    }

    Link { depth: CHAIN, next }
}

#[test]
pub fn long_sources() {
    let tree = long_chain().er();
    assert_eq!(tree.er_sources().count(), CHAIN + 1);
    assert!(tree.er_contains::<Deep>());

    let report = tree.er_report().to_string();
    assert_eq!(report.lines().count(), CHAIN + 2);
    assert!(report.contains("the buried cause"), "{report}");
    assert!(
        tree.er_report()
            .single_line()
            .to_string()
            .contains("the buried cause")
    );
}

#[test]
pub fn deep_tree_small_stack() {
    let handle = Builder::new()
        .stack_size(256 * 1024)
        .spawn(|| {
            let probes = Arc::new(AtomicUsize::new(0));
            let error = linear_chain(5_000, &probes);

            use fmt::Write as _;
            pub struct Sink {
                pub bytes: usize,
                pub lines: usize,
            }
            impl fmt::Write for Sink {
                fn write_str(&mut self, text: &str) -> fmt::Result {
                    self.bytes += text.len();
                    self.lines += text.bytes().filter(|&b| b == b'\n').count();

                    Ok(())
                }
            }

            let mut sink = Sink { bytes: 0, lines: 0 };
            probes.store(0, Ordering::Relaxed);
            write!(sink, "{}", error.er_report()).unwrap();

            assert_eq!(sink.lines, 5_000);
            assert!(sink.bytes >= 5_001 * "counted".len());
            assert_eq!(probes.load(Ordering::Relaxed), 5_001);

            let mut sink = Sink { bytes: 0, lines: 0 };
            probes.store(0, Ordering::Relaxed);
            write!(sink, "{}", error.er_report().single_line()).unwrap();

            assert_eq!(sink.lines, 0);
            assert!(sink.bytes >= 5_001 * "counted".len());
            assert_eq!(probes.load(Ordering::Relaxed), 5_001);

            assert!(error.er_find::<AbsentErr>().is_none());

            drop(error);
        })
        .expect("spawn");

    handle
        .join()
        .expect("a 5000 deep tree fits in a 256 KiB stack");
}

#[test]
pub fn wide_tree() {
    let probes = Arc::new(AtomicUsize::new(0));
    let branches: Vec<ErTree<Counted>> = (0..2_000)
        .map(|_| {
            Counted {
                probes: Arc::clone(&probes),
            }
            .er()
        })
        .collect();

    let error = ErTree::new(
        Counted {
            probes: Arc::clone(&probes),
        },
        branches,
    );

    assert_eq!(error.er_descendants().count(), 2_000);
    assert_eq!(error.er_report().to_string().lines().count(), 2_001);
}
