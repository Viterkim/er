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

        for layout in [Layout::Multiline, Layout::SingleLine] {
            let report = snapshot.er_report().layout(layout);
            let top = snapshot.er_top().layout(layout);
            assert_eq!(
                report.to_string(),
                tree.er_report().layout(layout).to_string()
            );
            assert_eq!(top.to_string(), tree.er_top().layout(layout).to_string());
            assert_eq!(report.er_report_string(), report.to_string());
            assert_eq!(top.er_top_string(), top.to_string());
            assert_eq!(top.er_report_string(), report.to_string());
            assert_eq!(report.er_top_string(), top.to_string());
            assert_eq!(report.er_top().layout, layout);
            assert_eq!(top.er_report().layout, layout);
            assert_eq!(report.er_report().layout, layout);
            assert_eq!(top.er_top().layout, layout);
            assert_eq!(report.er_snapshot(), snapshot);
            assert_eq!(top.er_snapshot(), snapshot);
            assert_eq!(format!("{report:?}"), report.to_string());
            assert_eq!(format!("{top:?}"), top.to_string());

            let owned = snapshot.clone().into_er_report().layout(layout);
            assert_eq!(owned.er_report_string(), report.er_report_string());
            assert_eq!(owned.er_top_string(), top.er_top_string());
            assert_eq!(owned.er_snapshot(), snapshot);
            assert_eq!(owned.er_entries().count(), snapshot.entries.len());
            assert_eq!(owned.er_top().layout, layout);

            let owned = owned.into_er_top();
            assert_eq!(owned.er_report_string(), report.er_report_string());
            assert_eq!(owned.er_top_string(), top.er_top_string());
            assert_eq!(owned.er_snapshot(), snapshot);
            assert_eq!(owned.er_report().layout, layout);
            assert_eq!(owned.into_er_report().layout, layout);

            let boxed = Box::new(snapshot.clone().into_er_report().layout(layout));
            assert_eq!(boxed.er_snapshot(), snapshot);
            assert_eq!(boxed.er_top_string(), top.er_top_string());
            assert_eq!(boxed.er_report_string(), report.er_report_string());
        }
        assert_eq!(snapshot.er_report_string(), tree.er_report_string());
        assert_eq!(snapshot.er_top_string(), tree.er_top_string());

        let mut renumbered = snapshot.clone();
        for entry in &mut renumbered.entries {
            entry.index += 85;
            entry.parent = entry.parent.map(|parent| parent + 85);
        }
        for layout in [Layout::Multiline, Layout::SingleLine] {
            assert_eq!(
                renumbered.er_report().layout(layout).to_string(),
                snapshot.er_report().layout(layout).to_string(),
            );
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

    fn check<E: Clone + IntoErSnapshot>(error: E, snapshot: &ErSnapshot, report: &str, top: &str) {
        assert_eq!(
            Err::<(), _>(error.clone()).er_report_string().unwrap_err(),
            report
        );
        assert_eq!(
            Err::<(), _>(error.clone()).er_top_string().unwrap_err(),
            top
        );
        assert_eq!(
            Err::<(), _>(error.clone())
                .er_report()
                .unwrap_err()
                .to_string(),
            report
        );
        assert_eq!(
            Err::<(), _>(error.clone())
                .er_top()
                .unwrap_err()
                .to_string(),
            top
        );
        assert_eq!(Err::<(), _>(error).er_snapshot().unwrap_err(), *snapshot);

        assert_eq!(Ok::<_, E>(85).er_report_string().unwrap(), 85);
        assert_eq!(Ok::<_, E>(85).er_top_string().unwrap(), 85);
        assert_eq!(Ok::<_, E>(85).er_report().unwrap(), 85);
        assert_eq!(Ok::<_, E>(85).er_top().unwrap(), 85);
        assert_eq!(Ok::<_, E>(85).er_snapshot().unwrap(), 85);
        assert_eq!(Ok::<_, E>(85).unwrap_report(), 85);
        assert_eq!(Ok::<_, E>(85).expect_report("saved failure"), 85);
    }

    let snapshot = ErTree::new(Message::new("top\nline"), [Message::new("child")]).er_snapshot();
    let report = snapshot.er_report_string();
    let top = snapshot.er_top_string();
    check(snapshot.clone(), &snapshot, &report, &top);
    check(&snapshot, &snapshot, &report, &top);
    assert!(std::ptr::eq(local_view(&snapshot).snapshot, &snapshot));
    assert!(std::ptr::eq(local_report(&snapshot).snapshot, &snapshot));

    let mut result = Err::<(), _>(snapshot.clone());
    assert_eq!(result.as_mut().er_snapshot().unwrap_err(), snapshot);
    assert_eq!(result.as_mut().er_top_string().unwrap_err(), top);

    for layout in [Layout::Multiline, Layout::SingleLine] {
        let report = snapshot.er_report().layout(layout);
        let top = snapshot.er_top().layout(layout);
        check(report, &snapshot, &report.to_string(), &top.to_string());
        check(top, &snapshot, &report.to_string(), &top.to_string());
        check::<&ErSnapshotReportRef<'_>>(
            &report,
            &snapshot,
            &report.to_string(),
            &top.to_string(),
        );
        check::<&ErSnapshotTopRef<'_>>(&top, &snapshot, &report.to_string(), &top.to_string());
        let owned_report = snapshot.clone().into_er_report().layout(layout);
        let owned_top = snapshot.clone().into_er_top().layout(layout);
        check(
            &owned_report,
            &snapshot,
            &report.to_string(),
            &top.to_string(),
        );
        check(&owned_top, &snapshot, &report.to_string(), &top.to_string());
        check(
            snapshot.clone().into_er_report().layout(layout),
            &snapshot,
            &report.to_string(),
            &top.to_string(),
        );
        check(
            snapshot.clone().into_er_top().layout(layout),
            &snapshot,
            &report.to_string(),
            &top.to_string(),
        );

        let error = Err::<(), _>(report).er_top().er_report().unwrap_err();
        assert_eq!(error.to_string(), report.to_string());

        let report: ErSnapshotReportRef<'_> = propagate(Err::<(), _>(top)).unwrap_err();
        let top: ErSnapshotTopRef<'_> = propagate(Err::<(), _>(report)).unwrap_err();
        assert!(std::ptr::eq(top.snapshot, &snapshot));
        assert_eq!(top.layout, layout);
    }

    let entries = snapshot.entries.as_ptr();
    let snapshot = Err::<(), _>(snapshot)
        .er_report()
        .er_top()
        .er_snapshot()
        .unwrap_err();
    assert_eq!(snapshot.entries.as_ptr(), entries);

    let result: Result<(), ErSnapshotReport> =
        propagate(Err::<(), _>(snapshot.into_er_top().single_line()));
    assert_eq!(result.as_ref().unwrap_err().layout, Layout::SingleLine);
    let result: Result<(), ErSnapshotTop> = propagate(result);
    assert_eq!(result.as_ref().unwrap_err().layout, Layout::SingleLine);
    let snapshot = result.er_snapshot().unwrap_err();
    assert_eq!(snapshot.entries.as_ptr(), entries);

    let mut result = Err::<(), _>(snapshot.into_er_top().single_line());
    let expected = result.as_ref().unwrap_err().er_report_string();
    let top = result.as_ref().unwrap_err().er_top_string();
    assert_eq!(result.as_ref().er_top().unwrap_err().to_string(), top);
    assert_eq!(result.as_mut().er_top().unwrap_err().to_string(), top);
    assert_eq!(result.as_ref().er_report_string().unwrap_err(), expected);
    assert_eq!(result.as_mut().er_report_string().unwrap_err(), expected);
    assert_eq!(
        result.as_ref().er_snapshot().unwrap_err(),
        result.as_ref().unwrap_err().snapshot
    );

    let mut result = result.er_report();
    assert_eq!(
        result.as_ref().er_report().unwrap_err().to_string(),
        expected
    );
    assert_eq!(
        result.as_mut().er_report().unwrap_err().to_string(),
        expected
    );
    assert_eq!(
        result.as_mut().er_report().unwrap_err().er_report_string(),
        expected
    );

    let entries = result.as_ref().unwrap_err().snapshot.entries.as_ptr();
    let mut boxed = Err::<(), _>(Box::new(result.unwrap_err()));
    assert_eq!(boxed.as_ref().er_report_string().unwrap_err(), expected);
    assert_eq!(boxed.as_mut().er_report_string().unwrap_err(), expected);
    assert_eq!(
        boxed.as_mut().er_report().unwrap_err().to_string(),
        expected
    );

    let snapshot = boxed.er_top().er_report().er_snapshot().unwrap_err();
    assert_eq!(snapshot.entries.as_ptr(), entries);

    let mut success = Ok::<_, ErSnapshot>(String::from("kept"));
    success
        .as_mut()
        .expect_report("saved failure")
        .push_str(" value");
    assert_eq!(success.unwrap_report(), "kept value");
}

#[test]
pub fn borrows() {
    use std::{ops::Deref, rc::Rc, sync::Arc};

    fn view(
        value: impl Deref<Target = ErSnapshot> + std::fmt::Display,
        snapshot: &ErSnapshot,
        text: &str,
    ) {
        assert!(std::ptr::eq(&*value, snapshot));
        assert_eq!(value.to_string(), text);
    }

    macro_rules! check {
        ($value:expr, $snapshot:expr, $report:expr, $top:expr $(,)?) => {{
            let snapshot: &ErSnapshot = $snapshot;
            let report: &str = $report;
            let top: &str = $top;
            let mut result = Err::<(), _>($value);
            view(result.as_ref().er_report().unwrap_err(), snapshot, report);
            view(result.as_mut().er_top().unwrap_err(), snapshot, top);
            assert_eq!(result.as_ref().er_report_string().unwrap_err(), report);
            assert_eq!(result.as_mut().er_report_string().unwrap_err(), report);

            let saved = result.as_mut().er_snapshot().unwrap_err();
            assert_eq!(saved, *snapshot);
            assert_ne!(saved.entries.as_ptr(), snapshot.entries.as_ptr());
        }};
    }

    let snapshot = ErTree::new(Message::new("top\nline"), [Message::new("child")]).er_snapshot();
    check!(
        &snapshot,
        &snapshot,
        &snapshot.er_report_string(),
        &snapshot.er_top_string(),
    );

    let shared = Arc::new(snapshot.clone());
    let local = Rc::new(snapshot.clone());
    check!(
        &shared,
        &shared,
        &snapshot.er_report_string(),
        &snapshot.er_top_string(),
    );
    check!(
        &local,
        &local,
        &snapshot.er_report_string(),
        &snapshot.er_top_string(),
    );

    for layout in [Layout::Multiline, Layout::SingleLine] {
        let report = snapshot.er_report().layout(layout);
        let top = snapshot.er_top().layout(layout);
        let expected = report.er_report_string();
        let expected_top = top.er_top_string();
        check!(&report, &snapshot, &expected, &expected_top);
        check!(&top, &snapshot, &expected, &expected_top);

        let mut borrowed = Err::<(), _>(&report);
        let view = borrowed.as_ref().er_report().unwrap_err();
        assert!(std::ptr::eq(view.snapshot, &snapshot));
        assert_eq!(view.to_string(), expected);
        assert_eq!(
            borrowed.as_mut().er_report().unwrap_err().to_string(),
            expected
        );

        let boxed = Box::new(report);
        let shared = Arc::new(report);
        let local = Rc::new(top);
        let field = ErShared::new(report);
        check!(&boxed, &snapshot, &expected, &expected_top);
        check!(&shared, &snapshot, &expected, &expected_top);
        check!(&local, &snapshot, &expected, &expected_top);
        check!(&field, &snapshot, &expected, &expected_top);
        assert_eq!(
            Err::<(), _>(&shared).er_report().unwrap_err().to_string(),
            expected
        );
        assert_eq!(
            Err::<(), _>(&local).er_top().unwrap_err().to_string(),
            expected_top
        );

        let shared = Arc::new(snapshot.clone().into_er_report().layout(layout));
        let local = Rc::new(snapshot.clone().into_er_top().layout(layout));
        check!(&shared, &shared.snapshot, &expected, &expected_top);
        check!(&local, &local.snapshot, &expected, &expected_top);

        let mut owned = snapshot.clone().into_er_report().layout(layout);
        check!(&owned, &owned.snapshot, &expected, &expected_top);
        let mut result = Err::<(), _>(&mut owned);
        let view = result.as_ref().er_report().unwrap_err();
        assert!(std::ptr::eq(
            view.snapshot,
            &result.as_ref().unwrap_err().snapshot
        ));
        assert_eq!(view.layout, layout);
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
    let entries = snapshot.er_report().er_entries();
    assert_eq!(entries.count(), snapshot.entries.len());

    let descendants = snapshot.er_top().er_descendants();
    assert_eq!(descendants.count(), 1);

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
