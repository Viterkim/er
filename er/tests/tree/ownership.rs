use core::{error::Error, fmt};
use er::*;
use std::io;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use std::thread::Builder;

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
    let sub = ErTree::new(StageErr, [fmt::Error]);
    let cause = io::Error::new(io::ErrorKind::TimedOut, "timeout");
    let inner = ErTree::new(LegacyErr { cause }, [sub]);
    let outer = inner.er(OuterErr::new);

    let below: Vec<_> = outer
        .er_descendants()
        .map(|node| node.error.to_string())
        .collect();
    assert_eq!(
        below,
        ["legacy", "StageErr", fmt::Error.to_string().as_str()]
    );
}

#[test]
pub fn borrowed_context() {
    let stored = Err::<String, _>(io::Error::other("offline").er_share());
    let source = stored.as_ref().unwrap_err();

    let first: ErTree<OuterErr> = source.er_with(|_: &io::Error| OuterErr);
    let second = source.er_with_tree(|_: &ErSharedTree<io::Error>| OuterErr);
    let third = stored.as_ref().er(()).unwrap_err();

    for tree in [first, second, third] {
        assert!(tree.er_contains_shared(source));
    }
}

#[test]
fn shared_identity() {
    let first = StageErr.er_share();
    let second = StageErr.er_share();
    let nested = er_new!(OuterErr, [&first]).er_share();
    let report = er_new!(OuterErr, [nested]);

    assert!(report.er_contains_shared(&first));
    assert!(!report.er_contains_shared(&second));
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
    drop(wrapped);

    assert_eq!(drops.load(Ordering::Relaxed), 3);
}

#[test]
pub fn shared_drop() {
    Builder::new()
        .stack_size(512 * 1024)
        .spawn(|| {
            let mut tree = OuterErr.er_share();
            let mut weak = Vec::new();

            for _ in 0..20_000 {
                weak.push(Arc::downgrade(&tree.tree));
                tree = tree.er(OuterErr::new).er_share();
            }

            drop(tree);
        })
        .unwrap()
        .join()
        .unwrap();
}
