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
        let tree = ErTree::new(source, [Message::new("healthy").er()]);
        let mut sources = tree.er_sources();
        assert_eq!(
            sources.by_ref().take(MAX_SOURCE_HOPS).count(),
            MAX_SOURCE_HOPS
        );
        assert!(!sources.truncated);
        assert!(sources.next().is_none());
        assert!(sources.truncated);

        assert!(tree.er_find::<fmt::Error>().is_none());
        assert_eq!(tree.er_find::<Message>().unwrap().text, "healthy");
        assert!(tree.er_find_all::<fmt::Error>().next().is_none());
        assert_eq!(
            tree.er_find_all::<Message>().next().unwrap().text,
            "healthy"
        );

        let entries: Vec<_> = tree.er_entries().collect();
        assert_eq!(entries.len(), MAX_SOURCE_HOPS + 2);
        assert!(entries[MAX_SOURCE_HOPS].source_truncated);
        assert_eq!(entries.last().unwrap().parent, Some(0));

        let multiline = tree.er_report().to_string();
        let single_line = tree.er_report().single_line().to_string();
        let saved = tree.er_snapshot();
        drop(tree);

        let stopped = &saved.entries[MAX_SOURCE_HOPS];
        assert!(stopped.source_truncated);
        assert_eq!(stopped.message, "loop");
        assert_eq!(saved.er_report().to_string(), multiline);
        assert_eq!(saved.er_report().single_line().to_string(), single_line);
        for output in [multiline, single_line] {
            assert_eq!(output.matches("[source limit reached]").count(), 1);
            assert!(output.contains("healthy"));
        }
    }

    let branch = ErTree::new(&SELF, [Message::new("healthy")]);
    let tree = ErTree::new(Message::new("root"), [branch]);
    assert!(tree.er_find::<fmt::Error>().is_none());
    assert_eq!(tree.nodes[0].er_find::<Message>().unwrap().text, "healthy");
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
pub fn boundaries() {
    for count in [MAX_SOURCE_HOPS - 1, MAX_SOURCE_HOPS, MAX_SOURCE_HOPS + 1] {
        let tree = chain(count).er();
        let expected = count.min(MAX_SOURCE_HOPS);
        let truncated = count > MAX_SOURCE_HOPS;

        let mut sources = tree.er_sources();
        assert_eq!(sources.by_ref().count(), expected);
        assert_eq!(sources.truncated, truncated);
        assert_eq!(tree.er_find_all::<Message>().count(), expected + 1);

        let entries: Vec<_> = tree.er_entries().collect();
        assert_eq!(entries.len(), expected + 1);
        assert_eq!(entries.last().unwrap().source_truncated, truncated);
        let report = tree.er_report().to_string();
        assert_eq!(report.contains("[source limit reached]"), truncated);
    }

    let tree = ErTree::new(chain(MAX_SOURCE_HOPS + 1), [chain(MAX_SOURCE_HOPS).er()]);
    assert_eq!(
        tree.er_find_all::<Message>().count(),
        2 * (MAX_SOURCE_HOPS + 1)
    );
    let saved = tree.er_snapshot();
    assert_eq!(saved.entries.len(), (MAX_SOURCE_HOPS + 1) * 2);
    let stopped = saved.er_entries().filter(|e| e.source_truncated).count();
    assert_eq!(stopped, 1);
    assert_eq!(saved.entries.last().unwrap().message, "cause");
}
