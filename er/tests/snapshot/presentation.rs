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
        let sibling = ErTree::from(Message::new("sibling"));
        let tree = ErTree::new(Message::new(message), [child, sibling]);
        let snapshot = tree.er_snapshot();
        let mut renumbered = snapshot.clone();
        for entry in &mut renumbered.entries {
            entry.index += 85;
            entry.parent = entry.parent.map(|parent| parent + 85);
        }

        for layout in [Layout::Multiline, Layout::SingleLine] {
            let report = snapshot.er_report().layout(layout).to_string();
            assert_eq!(report, tree.er_report().layout(layout).to_string());
            assert_eq!(renumbered.er_report().layout(layout).to_string(), report);
        }
    }
}

#[test]
pub fn results() {
    fn propagate<T, E, F: From<E>>(result: Result<T, E>) -> Result<T, F> {
        Ok(result?)
    }

    fn local_view(snapshot: &ErSnapshot) -> ErSnapshotReportRef<'_> {
        let mut result = Err::<(), _>(snapshot.er_top().single_line());
        result.as_mut().er_report().unwrap_err()
    }

    fn local_report(snapshot: &ErSnapshot) -> ErSnapshotReportRef<'_> {
        let mut result = Err::<(), _>(snapshot.er_report().single_line());
        result.as_mut().er_report().unwrap_err()
    }

    fn check<E: IntoErSnapshot>(error: E, report: &str) {
        assert_eq!(Err::<(), _>(error).er_report_string().unwrap_err(), report);
    }

    let snapshot = ErTree::new(Message::new("top\nline"), [Message::new("child")]).er_snapshot();
    check(snapshot.clone(), &snapshot.er_report_string());
    check(&snapshot, &snapshot.er_report_string());

    let expected = snapshot.er_report().single_line().to_string();
    for view in [local_view(&snapshot), local_report(&snapshot)] {
        assert_eq!(view.to_string(), expected);
    }

    for layout in [Layout::Multiline, Layout::SingleLine] {
        let report = snapshot.er_report().layout(layout);
        let top = snapshot.er_top().layout(layout);
        let expected = report.to_string();
        check(report, &expected);
        check(top, &expected);
        check::<&ErSnapshotReportRef<'_>>(&report, &expected);
        check::<&ErSnapshotTopRef<'_>>(&top, &expected);
        check::<&ErSnapshotReport>(&snapshot.clone().into_er_report().layout(layout), &expected);
        check::<&ErSnapshotTop>(&snapshot.clone().into_er_top().layout(layout), &expected);
        check(snapshot.clone().into_er_report().layout(layout), &expected);
        check(snapshot.clone().into_er_top().layout(layout), &expected);

        let report: ErSnapshotReportRef<'_> = propagate(Err::<(), _>(top)).unwrap_err();
        let top: ErSnapshotTopRef<'_> = propagate(Err::<(), _>(report)).unwrap_err();
        assert_eq!(top.er_report_string(), expected);
    }

    let saved = Err::<(), _>(snapshot.clone())
        .er_report()
        .er_top()
        .er_snapshot()
        .unwrap_err();
    let result: Result<(), ErSnapshotReport> =
        propagate(Err::<(), _>(saved.into_er_top().single_line()));
    let result: Result<(), ErSnapshotTop> = propagate(result);
    assert_eq!(result.as_ref().unwrap_err().layout, Layout::SingleLine);

    let boxed = Box::new(result.er_snapshot().unwrap_err().into_er_report());
    let saved = Err::<(), _>(boxed).er_top().er_snapshot().unwrap_err();
    assert_eq!(saved, snapshot);
}

#[test]
pub fn borrows() {
    use std::{fmt, ops::Deref, rc::Rc, sync::Arc};

    fn view(value: impl Deref<Target = ErSnapshot> + fmt::Display, text: &str) {
        assert_eq!(value.to_string(), text);
    }

    macro_rules! check {
        ($value:expr, $report:expr, $top:expr $(,)?) => {{
            let mut result = Err::<(), _>($value);
            view(result.as_ref().er_report().unwrap_err(), $report);
            view(result.as_mut().er_top().unwrap_err(), $top);
        }};
    }

    let snapshot = ErTree::new(Message::new("top\nline"), [Message::new("child")]).er_snapshot();
    let mut result = Err::<(), _>(&snapshot);
    let borrowed = result.as_mut().er_report().unwrap_err();
    assert!(core::ptr::eq(&*borrowed, &snapshot));

    let saved = result.er_snapshot().unwrap_err();
    assert_eq!(saved, snapshot);

    let report = snapshot.er_report_string();
    let top = snapshot.er_top_string();
    check!(&snapshot, &report, &top);
    check!(&Arc::new(snapshot.clone()), &report, &top);
    check!(&Rc::new(snapshot.clone()), &report, &top);

    for layout in [Layout::Multiline, Layout::SingleLine] {
        let report = snapshot.er_report().layout(layout);
        let top = snapshot.er_top().layout(layout);
        let expected = report.er_report_string();
        let expected_top = top.er_top_string();
        check!(&report, &expected, &expected_top);
        check!(&top, &expected, &expected_top);

        let boxed = Box::new(report);
        let shared = Arc::new(report);
        let local = Rc::new(top);
        let field = ErShared::new(report);
        check!(&boxed, &expected, &expected_top);
        check!(&shared, &expected, &expected_top);
        check!(&local, &expected, &expected_top);
        check!(&field, &expected, &expected_top);

        let shared = Arc::new(snapshot.clone().into_er_report().layout(layout));
        let local = Rc::new(snapshot.clone().into_er_top().layout(layout));
        check!(&shared, &expected, &expected_top);
        check!(&local, &expected, &expected_top);

        let mut owned = snapshot.clone().into_er_report().layout(layout);
        check!(&owned, &expected, &expected_top);

        let mut result = Err::<(), _>(&mut owned);
        assert_eq!(
            result.as_ref().er_report().unwrap_err().to_string(),
            expected
        );
        assert_eq!(result.as_mut().er_report_string().unwrap_err(), expected);
    }
}

#[test]
pub fn unwrap_messages() {
    fn panic_message(panic: Box<dyn std::any::Any + Send>) -> String {
        *panic.downcast::<String>().unwrap()
    }

    let snapshot = ErTree::new(Message::new("top\nline"), [Message::new("child")]).er_snapshot();
    let top = snapshot.er_top().single_line();
    let report = top.er_report_string();
    let panic = std::panic::catch_unwind(|| Err::<(), _>(top).unwrap_report()).unwrap_err();
    assert!(panic_message(panic).contains(&report));

    let report = snapshot.into_er_top().single_line();
    let expected = report.er_report_string();
    let panic = std::panic::catch_unwind(|| Err::<(), _>(report).expect_report("saved failure"))
        .unwrap_err();
    let message = panic_message(panic);
    assert!(message.contains("saved failure"));
    assert!(message.contains(&expected));
}

#[test]
pub fn callbacks() {
    let snapshot = ErTree::new(Message::new("top"), [Message::new("child")]).er_snapshot();
    let report = snapshot.er_report();
    let mut messages = Vec::new();
    snapshot
        .er_report()
        .er_for_each_entry(|entry| messages.push(entry.message.as_str()));
    assert_eq!(messages, ["top", "child"]);

    let mut lines = Vec::new();
    assert!(report.for_each_line(|l| lines.push(l.to_string())).is_ok());
    assert_eq!(lines.join("\n"), report.to_string());

    let failure = String::from("stop");
    let mut calls = 0;
    let result = report.try_for_each_line(|_| {
        calls += 1;
        Err(&failure)
    });
    assert_eq!(result, Err(ErLineError::Callback(&failure)));
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
