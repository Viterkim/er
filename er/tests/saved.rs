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

    assert_eq!(snapshot.saved.len(), tree.saved.len());
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

    let titles: Vec<_> = tree
        .saved
        .iter()
        .map(|saved| saved.title.as_str())
        .collect();
    assert_eq!(titles, ["first", "second", "third"]);
}

#[cfg(all(feature = "macros", target_has_atomic = "ptr"))]
#[test]
pub fn retained() {
    use er::*;
    use std::fmt::Error;

    let shared = er_new!(Error).er_save("first", "report").er_share();
    let outer = er_new!(Error, [&shared]).er_save("second", "extra");

    assert_eq!(outer.saved.len(), 2);
    assert_eq!(shared.tree.saved.len(), 1);
}
#[test]
pub fn saved_tree_result() {
    use er::*;
    use std::fmt;

    let result = Err::<(), _>(ErTree::from(fmt::Error)).er_save("request", |_| "sent");
    let borrowed = result.as_ref().er_report().unwrap_err();
    assert!(borrowed.ends_with("sent"));
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
