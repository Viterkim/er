use super::Message;
use er::*;

#[test]
pub fn same_as_live() {
    for message in [
        "85",
        "first\nsecond\n",
        "first\r\n\nsecond",
        "",
        "\n",
        "fake `- branch @ file:85",
    ] {
        let sub = ErTree::new(Message::new(message), [Message::new("last")]);
        let sibling = ErTree::from(Message::new("sibling"));
        let tree = ErTree::new(Message::new(message), [sub, sibling]);
        let snapshot = tree.er_snapshot();
        let mut renumbered = snapshot.clone();
        for entry in &mut renumbered.entries {
            entry.index += 85;
            entry.parent = entry.parent.map(|parent| parent + 85);
        }

        let report = snapshot.er_report();
        assert_eq!(report, tree.er_report());
        assert_eq!(renumbered.er_report(), report);
    }
}

#[test]
pub fn results() {
    let snapshot = ErTree::new(Message::new("top\nline"), [Message::new("sub")]).er_snapshot();
    let expected = snapshot.er_report();

    assert_eq!(Err::<(), _>(&snapshot).er_report().unwrap_err(), expected);
    assert_eq!(Err::<(), _>(snapshot).er_report().unwrap_err(), expected);
}

#[test]
pub fn top() {
    let snapshot = ErTree::new(Message::new("top"), [Message::new("sub")]).er_snapshot();

    assert_eq!(snapshot.top(), "top");
}

#[test]
pub fn unwrap_messages() {
    fn panic_message(panic: Box<dyn std::any::Any + Send>) -> String {
        *panic.downcast::<String>().unwrap()
    }

    let snapshot = ErTree::new(Message::new("top\nline"), [Message::new("sub")]).er_snapshot();
    let expected = snapshot.er_report();
    let panic =
        std::panic::catch_unwind(|| Err::<(), _>(snapshot).er_expect("saved failure")).unwrap_err();
    let message = panic_message(panic);

    assert!(message.contains("saved failure"));
    assert!(message.contains(&expected));
}

#[test]
pub fn lines() {
    let snapshot = ErTree::new(Message::new("top"), [Message::new("sub")]).er_snapshot();
    let mut lines = Vec::new();
    assert!(
        snapshot
            .for_each_line(|l| lines.push(l.to_string()))
            .is_ok()
    );
    assert_eq!(lines.join("\n"), snapshot.er_report());

    let failure = String::from("stop");
    let mut calls = 0;
    let result = snapshot.try_for_each_line(|_| {
        calls += 1;
        Err(&failure)
    });
    assert_eq!(result, Err(ErLineError::Callback(&failure)));
    assert_eq!(calls, 1);
}

#[test]
pub fn invalid_depth() {
    let mut snapshot = ErTree::new(Message::new("top"), [Message::new("sub")]).er_snapshot();

    for depth in [0, usize::MAX] {
        snapshot.entries[1].depth = depth;
        assert_eq!(snapshot.er_report(), "ER_INVALID_SNAPSHOT");
        let mut output = String::new();
        assert_eq!(
            er::er_internal::render::report::write_entries(&mut output, snapshot.er_entries()),
            Err(std::fmt::Error)
        );
    }
}
