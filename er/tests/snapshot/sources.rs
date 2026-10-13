use super::Message;
use core::{error::Error, fmt};
use er::walk::MAX_SOURCE_HOPS;
use er::*;
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Debug)]
pub struct Cycle {
    pub next: &'static Cycle,
    pub calls: AtomicUsize,
}
impl fmt::Display for Cycle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("loop")
    }
}
impl Error for Cycle {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        // Fail instead of hanging if the limit stops working.
        assert!(self.calls.fetch_add(1, Ordering::Relaxed) < MAX_SOURCE_HOPS * 16);
        Some(self.next)
    }
}

static SELF: Cycle = Cycle {
    next: &SELF,
    calls: AtomicUsize::new(0),
};
static LEFT: Cycle = Cycle {
    next: &RIGHT,
    calls: AtomicUsize::new(0),
};
static RIGHT: Cycle = Cycle {
    next: &LEFT,
    calls: AtomicUsize::new(0),
};

#[test]
pub fn cycles() {
    for source in [&SELF, &LEFT] {
        let tree = ErTree::new(source, [ErTree::from(Message::new("healthy"))]);
        let mut sources = tree.er_sources();
        assert_eq!(sources.by_ref().count(), MAX_SOURCE_HOPS);
        assert!(sources.truncated);

        let report = tree.er_report().to_string();
        assert_eq!(report.matches("[source limit reached]").count(), 1);
        assert!(report.contains("healthy"));

        assert_eq!(tree.er_snapshot().er_report().to_string(), report);
    }
}

pub fn chain(sources: usize) -> Message {
    let mut error = Message::new("cause");
    for _ in 0..sources {
        error = Message {
            text: "link",
            source: Some(Box::new(error)),
        };
    }
    error
}

#[test]
pub fn native_errors() {
    let error = chain(1);
    assert_eq!(error.er_top_string(), "link");
    assert_eq!(error.er_report_string(), "link\n`- cause");

    let snapshot = error.er_snapshot();
    assert_eq!(snapshot.er_report_string(), error.er_report_string());
    assert_eq!(snapshot.entries[1].kind, ErEntryKind::Source);

    let error: BoxError = Box::new(error);
    assert_eq!(error.er_snapshot(), snapshot);
}

#[test]
pub fn boundaries() {
    for count in [MAX_SOURCE_HOPS - 1, MAX_SOURCE_HOPS, MAX_SOURCE_HOPS + 1] {
        let tree = ErTree::from(chain(count));
        let expected = count.min(MAX_SOURCE_HOPS);
        let truncated = count > MAX_SOURCE_HOPS;

        let mut sources = tree.er_sources();
        assert_eq!(sources.by_ref().count(), expected);
        assert_eq!(sources.truncated, truncated);

        let report = tree.er_report().to_string();
        assert_eq!(report.contains("[source limit reached]"), truncated);
    }

    let tree = ErTree::new(
        chain(MAX_SOURCE_HOPS + 1),
        [ErTree::from(chain(MAX_SOURCE_HOPS))],
    );
    let saved = tree.er_snapshot();
    let stopped = saved.er_entries().filter(|e| e.source_truncated).count();
    assert_eq!(stopped, 1);
}
