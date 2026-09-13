use super::rendering::Message;
use er::*;

#[test]
fn paths() {
    for (file, small) in [
        (
            "/home/viktor/sparmock-engine-unreal-killer/src/lib.rs",
            "sparmock-engine-unreal-killer/src/lib.rs",
        ),
        (r"D:\Hyggekode\sparlock\src\lib.rs", r"sparlock\src\lib.rs"),
        ("/a/src/blabla/src/b/src/lib.rs", "b/src/lib.rs"),
        ("/home/viktor/src/kragmos/src/lib.rs", "kragmos/src/lib.rs"),
        (r"D:\src\sparlock\src\lib.rs", r"sparlock\src\lib.rs"),
        (r"\\server\share\kragmos\src\lib.rs", r"kragmos\src\lib.rs"),
        ("/src/lib.rs", "src/lib.rs"),
        (r"C:\src\lib.rs", r"src\lib.rs"),
        ("src/lib.rs", "src/lib.rs"),
        ("workspace/kragmos/src/lib.rs", "kragmos/src/lib.rs"),
        (
            "/håndboldfuglen/src/ダンガンロンパ 希望の学園と絶望の高校生.rs",
            "håndboldfuglen/src/ダンガンロンパ 希望の学園と絶望の高校生.rs",
        ),
        ("/work//sparlock//src/lib.rs", "sparlock//src/lib.rs"),
        ("/work/src-tools/lib.rs", "/work/src-tools/lib.rs"),
        ("/work/examples/main.rs", "/work/examples/main.rs"),
        ("/work/kragmos/src", "/work/kragmos/src"),
        ("", ""),
    ] {
        let mut snapshot = Message("error").er().er_snapshot();
        let location = ErSnapshotLocation {
            file: file.to_owned(),
            line: 7,
            column: 9,
        };
        snapshot.entries[0].src_location = Some(location.clone());
        let printed = if cfg!(feature = "small_path_src") {
            small
        } else {
            file
        };
        let expected = format!("error @ {printed}:7:9");

        for layout in [Layout::Multiline, Layout::SingleLine] {
            let report = snapshot.er_report().layout(layout);
            assert_eq!(report.to_string(), expected);
        }
        assert_eq!(snapshot.entries[0].src_location, Some(location));
    }
}

#[test]
fn live_and_saved() -> core::fmt::Result {
    let message = "/keep/message/src/as-written";
    let tree = Message(message).er();
    let location = tree.src_location;
    assert_eq!(location.file(), file!());
    let printed = if cfg!(feature = "small_path_src") {
        "format/src/paths.rs"
    } else {
        file!()
    };
    let expected = format!(
        "{message} @ {printed}:{}:{}",
        location.line(),
        location.column()
    );
    let snapshot = tree.er_snapshot();
    assert_eq!(
        snapshot.entries[0]
            .src_location
            .as_ref()
            .map(|location| location.file.as_str()),
        Some(file!())
    );

    for layout in [Layout::Multiline, Layout::SingleLine] {
        let live = tree.er_report().layout(layout);
        let saved = snapshot.er_report().layout(layout);
        assert_eq!(live.to_string(), expected);
        assert_eq!(saved.to_string(), expected);
        let mut lines = Vec::new();
        live.for_each_line(|line| lines.push(line.to_owned()))?;
        assert_eq!(lines, [expected.as_str()]);
    }
    assert_eq!(tree.er_top().to_string(), message);
    Ok(())
}
