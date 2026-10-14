use core::{error::Error, fmt};
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
pub fn owned_context() {
    let child = ErTree::new(StageErr, [fmt::Error]);
    let cause = io::Error::new(io::ErrorKind::TimedOut, "timeout");
    let inner = ErTree::new(LegacyErr { cause }, [child]);
    let outer = inner.er(OuterErr::new);

    let children: Vec<_> = outer
        .er_descendants()
        .map(|node| node.error.to_string())
        .collect();
    assert_eq!(
        children,
        ["legacy", "StageErr", fmt::Error.to_string().as_str()]
    );
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

        let wrapped = StageErrWrap::from(ErTree::new(StageErr, [error]));
        let recovered = wrapped.tree;
        let report = recovered.into_er_report();

        if top {
            drop(report.tree.into_er_top());
        } else {
            drop(report);
        }

        assert_eq!(drops.load(Ordering::Relaxed), 3);
    }
}
