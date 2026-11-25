use super::Message;
use er::*;

#[test]
pub fn saved_tree() {
    let source = Some(Box::new(Message::new("native source")));
    let root = Message {
        text: "HaandboldFuglen",
        source,
    };
    let sub = ErTree::from(Message::new("sub")).er::<Message>(|| Message::new("parent"));
    let tree = ErTree::new(root, [sub]);
    let snapshot = tree.er_snapshot();

    for (live, saved) in tree.er_entries().zip(snapshot.er_entries()) {
        assert_eq!(
            (&*saved.message, saved.kind, saved.index, saved.parent),
            (&*live.error.to_string(), live.kind, live.index, live.parent),
        );
    }

    let expected = tree.er_report().to_string();
    drop(tree);

    assert_eq!(snapshot.er_report().to_string(), expected);
}

#[test]
pub fn broken_formatter() {
    let source = Some(Box::new(Message::new("native source")));
    let root = Message {
        text: "broken",
        source,
    };
    let sub = ErTree::from(Message::new("sub")).er::<Message>(|| Message::new("broken"));
    let tree = ErTree::new(root, [sub]);
    let snapshot = tree.er_snapshot();
    let messages: Vec<_> = snapshot
        .er_entries()
        .map(|entry| entry.message.as_str())
        .collect();

    assert_eq!(
        messages,
        ["ER_FMT_FAILED", "native source", "ER_FMT_FAILED", "sub"]
    );

    // A broken message only loses itself, everything below it still prints.
    let (_, report) = Err::<(), _>(tree).er_split().unwrap_err();

    assert!(report.contains("ER_FMT_FAILED") && report.contains("sub"));
}

#[test]
pub fn deep_tree() {
    let mut tree = ErTree::from(Message::new("HaandboldFuglen"));
    for _ in 0..20_000 {
        tree = tree.er(|| Message::new("HaandboldFuglen"));
    }

    let snapshot = tree.er_snapshot();
    drop(tree);

    // The report is 600 MB of indentation at this depth, so count the lines as they come.
    let mut lines = 0;
    snapshot.for_each_line(|_| lines += 1).unwrap();
    assert_eq!(lines, 20_001);
}
