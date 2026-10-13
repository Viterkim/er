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
    let mut error = ErTree::from(Counted {
        probes: Arc::clone(probes),
    });

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
    let error = ErTree::new(NeverFormat, [ErTree::from(NeverFormat)]);
    let error = ErTree::new(NeverFormat, [error, ErTree::from(NeverFormat)]);

    assert!(error.er_find::<AbsentErr>().is_none());
    assert!(error.er_contains::<NeverFormat>());
    let _ = error.er_descendants().count();
    let _ = error.er_report().er_entries().count();

    let _top = error.er_top();
    let _report = error.er_report();

    let _owned = error.into_er_report();
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
