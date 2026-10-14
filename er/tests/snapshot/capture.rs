use super::Message;
use er::*;

#[test]
pub fn saved_tree() {
    let source = Some(Box::new(Message::new("native source")));
    let root = Message {
        text: "HaandboldFuglen",
        source,
    };
    let child = ErTree::from(Message::new("child")).er::<Message>(|| Message::new("parent"));
    let tree = ErTree::new(root, [child]);
    let snapshot = tree.er_snapshot();
    assert_eq!(tree.er_report().er_snapshot(), snapshot);

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
    let child = ErTree::from(Message::new("child")).er::<Message>(|| Message::new("broken"));
    let tree = ErTree::new(root, [child]);
    let snapshot = tree.er_snapshot();
    let messages: Vec<_> = snapshot
        .er_entries()
        .map(|entry| entry.message.as_str())
        .collect();

    assert_eq!(
        messages,
        ["ER_FMT_FAILED", "native source", "ER_FMT_FAILED", "child"]
    );

    for text in [
        tree.top.er_top_string(),
        tree.top.er_report_string(),
        tree.er_top_string(),
        tree.er_report_string(),
        tree.er_top().single_line().er_report_string(),
        tree.er_report().single_line().er_top_string(),
    ] {
        assert_eq!(text, "ER_FMT_FAILED");
    }

    let top = tree.into_er_report().single_line().into_er_top();

    #[cfg(feature = "macros")]
    {
        #[derive(Er)]
        #[er(format = "{report}")]
        struct Saved {
            #[er(into_top)]
            kind: Message,
            #[er(into_report_string)]
            report: String,
        }

        let result: Result<(), Saved> = Err::<(), _>(top).er_into(|report| report);
        let saved = result.unwrap_err();
        assert_eq!(saved.report, "ER_FMT_FAILED");
        assert_eq!(saved.kind.text, "broken");
    }

    #[cfg(not(feature = "macros"))]
    assert_eq!(
        Err::<(), _>(top).er_report_string().unwrap_err(),
        "ER_FMT_FAILED"
    );
}

#[test]
pub fn deep_tree() {
    let mut tree = ErTree::from(Message::new("HaandboldFuglen"));
    for _ in 0..20_000 {
        tree = tree.er(|| Message::new("HaandboldFuglen"));
    }

    let snapshot = tree.er_snapshot();
    drop(tree);

    assert_eq!(
        snapshot
            .er_report()
            .single_line()
            .to_string()
            .matches("HaandboldFuglen")
            .count(),
        20_001
    );
}
