#[cfg(all(feature = "stack_traces", feature = "macros"))]
#[test]
pub fn traces_only_on_err() {
    use er::*;
    use std::{backtrace::Backtrace, fmt};

    #[derive(Er)]
    struct Top;

    let ok: ErResult<(), Top> =
        Ok::<(), fmt::Error>(()).er(|_| ().er_save("trace", || -> Backtrace { panic!() }));
    assert!(ok.is_ok());

    let tree: ErTree<Top> = Err::<(), _>(fmt::Error)
        .er(|_| ().er_save("trace", Backtrace::force_capture))
        .unwrap_err();
    assert!(matches!(tree.saved[0].kind, ErSavedKind::StackTrace(_)));
}

#[cfg(feature = "stack_traces")]
#[test]
pub fn snapshot_report() {
    use er::ErTree;
    use std::{backtrace::Backtrace, fmt};

    let mut tree = ErTree::from(fmt::Error).er_save("trace", Backtrace::force_capture());
    tree.saved.push(tree.saved[0].clone());
    let tree = tree
        .er_save("trace", Backtrace::disabled())
        .er_save("trace", Backtrace::disabled());
    let snapshot = tree.er_snapshot();

    assert_eq!(snapshot.er_report(), tree.er_report());
}

#[cfg(feature = "macros")]
#[test]
pub fn travels_up() {
    use er::*;
    use std::fmt;

    #[derive(Er)]
    struct Top;

    let first = er_new!(fmt::Error).er_save("first", "report");
    let second = Err::<(), _>(fmt::Error.er_save("second", "report"));
    let tree: ErTree<Top> =
        er_all!(|_| ().er_save("third", "report"), [first, second]).unwrap_err();

    let titles: Vec<_> = tree.er_saved().map(|saved| saved.title.as_str()).collect();
    assert_eq!(titles, ["third", "first", "second"]);
}

#[cfg(feature = "macros")]
#[test]
pub fn branch_report() {
    use er::*;

    #[derive(Er)]
    struct ImportErr;
    #[derive(Er)]
    struct ParseErr;

    let branch = ImportErr.er_add(ParseErr).er_save("input", "data");
    let expected = "ImportErr\n|- ParseErr\n`- input [saved]\n     data";
    assert_eq!(branch.er_report(), expected);
    assert_eq!(branch.er_snapshot().er_report(), expected);

    let branch: ErBranch<ImportErr> = ImportErr.er_save("input", "data").er_add(ParseErr);
    assert_eq!(branch.er_report(), expected);

    let result = Err::<(), _>(branch);
    let with_saves = "|- ParseErr\n|- input [saved]\n|    data\n`- device [saved]\n     disk full";
    let borrowed = result.as_ref().er_save("device", |_| "disk full");
    assert_eq!(
        borrowed.er_report().unwrap_err().split_once('\n'),
        Some(("ImportErr", with_saves))
    );

    let report = result
        .er_save("device", |_| "disk full")
        .er_report()
        .unwrap_err();
    let (_, children) = report.split_once('\n').unwrap();
    assert_eq!(children, with_saves);
}

#[cfg(all(feature = "macros", target_has_atomic = "ptr"))]
#[test]
pub fn retained() {
    use er::*;
    use std::fmt::Error;

    let shared = er_new!(Error).er_save("first", "report").er_share();
    let outer = er_new!(Error, [&shared, &shared]).er_save("second", "extra");

    let report = outer.er_report();
    assert_eq!(report.matches("first [saved]").count(), 2);
    assert_eq!(outer.er_saved().count(), 3);
    assert_eq!(outer.er_snapshot().er_saved().count(), 3);
}

#[test]
pub fn attachments_follow_their_errors() {
    use er::*;
    use std::fmt::Error;

    let first = ErTree::from(Error).er_save("request", "sent");
    let second = ErTree::from(Error)
        .er_save("request", "sent")
        .er_save("request", "sent")
        .er_save("retry", "receiving");
    let tree = ErTree::new(Error, [first, second]);
    let report = tree.er_report();
    let attachments: Vec<_> = report
        .lines()
        .filter(|line| line.ends_with(" [saved]"))
        .collect();

    assert_eq!(
        attachments,
        [
            "|  `- request [saved]",
            "   |- request [saved]",
            "   `- retry [saved]"
        ]
    );
    let snapshot = tree.er_snapshot();
    assert_eq!(snapshot.entries[2].saved.len(), 2);
    assert_eq!(snapshot.er_report(), report);
}

#[test]
fn saved_snapshots_keep_their_rows() {
    use er::*;
    use std::fmt::Error;

    let storage = ErTree::from(Error)
        .er_save("device", "disk full")
        .er_snapshot();
    let worker = ErTree::from(Error)
        .er_save("storage", storage)
        .er_snapshot();
    let app = ErTree::from(Error).er_save("worker", worker).er_snapshot();

    let ErSavedKind::Snapshot(worker) = &app.entries[0].saved[0].kind else {
        panic!()
    };
    let ErSavedKind::Snapshot(storage) = &worker.entries[0].saved[0].kind else {
        panic!()
    };
    assert_eq!(storage.entries[0].saved[0].title, "device");
}

#[test]
fn append_to_snapshot() {
    use er::*;
    use std::fmt::Error;

    let child = ErTree::from(Error).er_save("child", "report");
    let snapshot = ErTree::new(Error, [child])
        .er_save("first", "a")
        .er_save("second", "b")
        .er_snapshot();

    let result = Err::<(), _>(snapshot).er_save("third", |_| "c");
    let report = result.as_ref().er_report().unwrap_err();
    assert_eq!(result.er_snapshot().unwrap_err().er_report(), report);
}

#[test]
pub fn nested_attachment_body() {
    use er::*;
    use std::fmt::Error;

    let tree = ErTree::from(Error).er_save(
        "previous",
        "previous\r\n`- inner [saved]\r\n     data\r\n\r\n     after\r\n",
    );

    assert!(tree.er_report().ends_with(
        "`- previous [saved]\n     previous\n     `- inner [saved]\n          data\n\n          after"
    ));
}

#[test]
fn moved_nodes_keep_their_attachments() {
    use er::*;
    use std::{fmt, io};

    let first = ErTree::from(io::Error::other("first")).er_save("request", "sent");
    let second = ErTree::from(io::Error::other("second")).er_save("request", "received");
    let mut tree = ErTree::new(fmt::Error, [first, second]);

    tree.nodes.swap(0, 1);
    let swapped = tree.er_report();
    let rows: Vec<_> = swapped.lines().collect();
    assert!(rows[1].contains("second"));
    assert_eq!(rows[3], "|       received");
    assert!(rows[4].contains("first"));
    assert_eq!(rows[6], "        sent");

    drop(tree.nodes.remove(0));
    let remaining = tree.er_report();
    assert!(remaining.ends_with("   `- request [saved]\n        sent"));
    assert!(!remaining.contains("received"));
}

#[test]
pub fn saved_tree_result() {
    use er::*;
    use std::fmt;

    let child = fmt::Error.er_save("retry", "could not retry");
    let tree = ErTree::new(fmt::Error, [child]).er_save("request", "sent");
    let result = Err::<(), _>(tree).er_save("response", |_| "received");
    let borrowed = result.as_ref().er_report().unwrap_err();
    let snapshot = result.as_ref().er_snapshot().unwrap_err();

    assert_eq!(snapshot.er_report(), borrowed);
    assert_eq!(borrowed, result.er_report().unwrap_err());
}

#[test]
pub fn saved_snapshot_result() {
    use er::*;
    use std::fmt;

    let snapshot = ErTree::from(fmt::Error).er_snapshot();
    let result = Err::<(), _>(snapshot).er_save("request", |_| "sent");
    assert!(result.er_report().unwrap_err().ends_with("sent"));
}

#[test]
pub fn saved_snapshot_dedup() {
    use er::*;
    use std::fmt;

    let snapshot = ErTree::from(fmt::Error)
        .er_save("request", "sent")
        .er_snapshot();
    let expected = snapshot.er_report();
    let result = Err::<(), _>(snapshot).er_save("request", |_| "sent");
    let borrowed = result.as_ref().er_report().unwrap_err();
    let owned = result.er_report().unwrap_err();

    assert_eq!([borrowed, owned], [expected.clone(), expected]);
}
