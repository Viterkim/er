use super::Message;
use er::*;

#[test]
pub fn layouts() {
    for message in [
        "85",
        "first\nsecond\n",
        "first\r\n\nsecond",
        "",
        "\n",
        "fake `- branch @ file:85",
    ] {
        let child = ErTree::new(Message::new(message), [Message::new("last")]);
        let sibling = Message::new("sibling").er();
        let tree = ErTree::new(Message::new(message), [child, sibling]);
        let snapshot = tree.er_snapshot();

        for layout in [Layout::Multiline, Layout::SingleLine] {
            let report = snapshot.er_report().layout(layout);
            let top = snapshot.er_top().layout(layout);
            assert_eq!(
                report.to_string(),
                tree.er_report().layout(layout).to_string()
            );
            assert_eq!(top.to_string(), tree.er_top().layout(layout).to_string());
            assert_eq!(format!("{report:?}"), report.to_string());
            assert_eq!(format!("{top:?}"), top.to_string());
        }
    }
}

#[test]
pub fn callbacks() {
    let snapshot = ErTree::new(Message::new("top"), [Message::new("child")]).er_snapshot();
    let report = snapshot.er_report();
    let mut messages = Vec::new();
    report.er_for_each_entry(|entry| messages.push(entry.message.as_str()));
    assert_eq!(messages, ["top", "child"]);

    let mut lines = Vec::new();
    assert!(
        report
            .for_each_line(|line| lines.push(line.to_string()))
            .is_ok()
    );
    assert_eq!(lines.join("\n"), report.to_string());

    let failure = String::from("stop");
    let mut calls = 0;
    let result = report.try_for_each_line(|_| {
        calls += 1;
        Err(&failure)
    });
    assert_eq!(result, Err(LineError::Callback(&failure)));
    assert_eq!(calls, 1);
}

#[test]
pub fn invalid_depth() {
    let mut snapshot = ErTree::new(Message::new("top"), [Message::new("child")]).er_snapshot();

    for depth in [0, usize::MAX] {
        snapshot.entries[1].depth = depth;
        for layout in [Layout::Multiline, Layout::SingleLine] {
            assert_eq!(
                snapshot.er_report().layout(layout).to_string(),
                "ER_INVALID_SNAPSHOT"
            );
            let mut output = String::new();
            assert_eq!(
                er::render::report::write_entries(&mut output, snapshot.er_entries(), layout),
                Err(std::fmt::Error)
            );
        }
    }
}
