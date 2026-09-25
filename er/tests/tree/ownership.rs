use core::{error::Error, fmt, ptr};
use er::*;
use std::io;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

#[derive(Er)]
#[er(wrap)]
pub struct StageErr;

#[derive(Er)]
pub struct OuterErr;

#[derive(Debug)]
pub struct LegacyErr {
    pub cause: io::Error,
}
impl fmt::Display for LegacyErr {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("legacy")
    }
}
impl Error for LegacyErr {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.cause)
    }
}

#[test]
pub fn native_source() {
    let cause = io::Error::new(io::ErrorKind::TimedOut, "timeout");
    let error = ErTree::new(OuterErr, [LegacyErr { cause }]);

    assert_eq!(error.er_descendants().count(), 1);

    let source = error.er_find::<io::Error>().unwrap();
    assert!(ptr::eq(
        source,
        &error.er_find::<LegacyErr>().unwrap().cause
    ));
    #[cfg(feature = "src_locations")]
    assert_eq!(error.nodes[0].src_location, error.src_location);

    let report = error.er_report().to_string();
    assert_eq!(report.lines().last().unwrap().trim(), "`- timeout");
}

#[test]
pub fn owned_context() {
    let grandchild = ErTree::from(fmt::Error);
    #[cfg(feature = "src_locations")]
    let grandchild_src_location = grandchild.src_location;

    let child = ErTree::new(StageErr, [grandchild]);
    #[cfg(feature = "src_locations")]
    let child_src_location = child.src_location;

    let cause = io::Error::new(io::ErrorKind::TimedOut, "timeout");
    let inner = ErTree::new(LegacyErr { cause }, [child]);

    assert_eq!(
        inner
            .er_sources()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
        ["timeout"]
    );
    #[cfg(feature = "src_locations")]
    let inner_src_location = inner.src_location;

    let expected_line = line!() + 1;
    let outer = inner.er::<OuterErr>(());

    assert!(outer.er_contains::<LegacyErr>());
    assert!(outer.er_contains::<io::Error>());
    let children: Vec<_> = outer
        .er_descendants()
        .map(|node| node.error.to_string())
        .collect();
    assert_eq!(
        children,
        ["legacy", "StageErr", fmt::Error.to_string().as_str()]
    );
    #[cfg(feature = "src_locations")]
    {
        let child = &outer.nodes[0].nodes[0];
        assert_eq!(outer.src_location.line(), expected_line);
        assert_eq!(outer.nodes[0].src_location, inner_src_location);
        assert_eq!(child.src_location, child_src_location);
        assert_eq!(child.nodes[0].src_location, grandchild_src_location);
    }

    let _ = expected_line;
}

pub struct Tracked {
    pub drops: Arc<AtomicUsize>,
}
impl fmt::Display for Tracked {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("tracked")
    }
}
impl fmt::Debug for Tracked {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("Tracked")
    }
}
impl Error for Tracked {}
impl Drop for Tracked {
    fn drop(&mut self) {
        self.drops.fetch_add(1, Ordering::Relaxed);
    }
}

#[test]
pub fn drop_once() {
    pub fn shared<T: Send + Sync>() {}
    shared::<ErTree<StageErr>>();
    shared::<StageErrWrap>();

    for top in [false, true] {
        let drops = Arc::new(AtomicUsize::new(0));
        let first = ErTree::from(Tracked {
            drops: Arc::clone(&drops),
        });
        let second = ErTree::from(Tracked {
            drops: Arc::clone(&drops),
        });

        let error = ErTree::new(
            Tracked {
                drops: Arc::clone(&drops),
            },
            [first.into_er_part(), second.into_er_part()],
        );

        assert_eq!(drops.load(Ordering::Relaxed), 0);

        let wrapped = StageErrWrap::from(ErTree::new(StageErr, [error]));
        let recovered = wrapped.tree;
        let report = recovered.into_er_report();

        assert_eq!(report.tree.er_descendants().count(), 3);
        assert_eq!(drops.load(Ordering::Relaxed), 0);

        if top {
            drop(report.tree.into_er_top());
        } else {
            drop(report);
        }

        assert_eq!(drops.load(Ordering::Relaxed), 3);
    }
}
