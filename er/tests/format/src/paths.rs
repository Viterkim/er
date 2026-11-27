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
        let mut snapshot = ErTree::from(Message("error")).er_snapshot();
        let location = ErSnapshotLocation {
            file: file.into(),
            line: 7,
            column: 9,
        };
        snapshot.entries[0].src_location = Some(location);
        let printed = if cfg!(feature = "small_path_src") {
            small
        } else {
            file
        };
        let expected = format!("error @ {printed}:7:9");

        assert_eq!(snapshot.er_report().to_string(), expected);
    }
}

#[test]
fn live_and_saved() {
    let message = "/keep/message/src/as-written";
    let mut tree = ErTree::from(Message(message));
    let location = core::panic::Location::caller();
    tree.src_location = Some(location);
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
            .map(|location| location.file.as_ref()),
        Some(file!())
    );

    assert_eq!(tree.er_report().to_string(), expected);
    assert_eq!(snapshot.er_report().to_string(), expected);
    assert_eq!(tree.top.to_string(), message);
}

#[test]
fn line_and_column() {
    let mut snapshot = ErTree::from(Message("error")).er_snapshot();
    for (line, column) in [(0, 0), (u32::MAX, u32::MAX)] {
        snapshot.entries[0].src_location = Some(ErSnapshotLocation {
            file: "lib.rs".into(),
            line,
            column,
        });

        assert!(snapshot.er_report().ends_with(&format!(":{line}:{column}")));
    }
}

#[test]
fn dependency_paths() {
    for (file, normal, small) in [
        (
            "/home/viktor/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aktor-0.0.13/src/setup/impls.rs",
            "aktor-0.0.13/src/setup/impls.rs",
            "aktor-0.0.13/src/setup/impls.rs",
        ),
        (
            r"C:\Users\viktor\.cargo\registry\src\index.crates.io-1949cf8c6b5b557f\aktor-0.0.13\src\setup.rs",
            r"aktor-0.0.13\src\setup.rs",
            r"aktor-0.0.13\src\setup.rs",
        ),
        (
            "~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aktor-0.0.13/examples/main.rs",
            "aktor-0.0.13/examples/main.rs",
            "aktor-0.0.13/examples/main.rs",
        ),
        (
            ".cargo/registry/src/-378542c1a6cac901/local-0.1.0/src/lib.rs",
            "local-0.1.0/src/lib.rs",
            "local-0.1.0/src/lib.rs",
        ),
        (
            "registry/src/index.crates.io-1949cf8c6b5b557f/aktor-0.0.13/src/lib.rs",
            "aktor-0.0.13/src/lib.rs",
            "aktor-0.0.13/src/lib.rs",
        ),
        (
            r"git\checkouts\bitcode-b520b407855ee695\0123456\src\lib.rs",
            r"bitcode\src\lib.rs",
            r"bitcode\src\lib.rs",
        ),
        (
            "build/deps/git/checkouts/aktor-8e4580d2b99937f4/0123456/src/lib.rs",
            "aktor/src/lib.rs",
            "aktor/src/lib.rs",
        ),
        (
            "/home/viktor/.cargo/git/checkouts/bitcode-b520b407855ee695/0123456/src/lib.rs",
            "bitcode/src/lib.rs",
            "bitcode/src/lib.rs",
        ),
        (
            "/home/viktor/.cargo/git/checkouts/aktor-8e4580d2b99937f4/0123456/aktor/src/setup.rs",
            "aktor/src/setup.rs",
            "aktor/src/setup.rs",
        ),
        (
            r"C:\Users\viktor\.cargo\git\checkouts\bitcode-b520b407855ee695\0123456\src\lib.rs",
            r"bitcode\src\lib.rs",
            r"bitcode\src\lib.rs",
        ),
        (
            "/home/viktor/.cargo/git/checkouts/monorepo-b520b407855ee695/deadbee/engine/src/lib.rs",
            "monorepo/engine/src/lib.rs",
            "engine/src/lib.rs",
        ),
        (
            r"/home/viktor/.cargo/git/checkouts/monorepo-b520b407855ee695/deadbee/engine/src\lib.rs",
            r"monorepo/engine/src\lib.rs",
            r"engine/src\lib.rs",
        ),
        (
            r"/home/viktor/.cargo/git/checkouts/monorepo-b520b407855ee695/deadbee/engine\src/lib.rs",
            r"monorepo/engine\src/lib.rs",
            r"engine\src/lib.rs",
        ),
        (
            "/home/viktor/.cargo/git/checkouts/monorepo-b520b407855ee695/deadbee/engine/src/src/parser.rs",
            "monorepo/engine/src/src/parser.rs",
            "src/src/parser.rs",
        ),
        (
            "/home/viktor/.cargo/git/checkouts/monorepo-b520b407855ee695/deadbee/engine/src",
            "monorepo/engine/src",
            "monorepo/engine/src",
        ),
        (
            "/work/checkouts/bitcode-b520b407855ee695/0123456/src/lib.rs",
            "/work/checkouts/bitcode-b520b407855ee695/0123456/src/lib.rs",
            "0123456/src/lib.rs",
        ),
        (
            "/home/viktor/work/registry/src/foo/bar.rs",
            "/home/viktor/work/registry/src/foo/bar.rs",
            "registry/src/foo/bar.rs",
        ),
    ] {
        let mut snapshot = ErTree::from(Message("error")).er_snapshot();
        snapshot.entries[0].src_location = Some(ErSnapshotLocation {
            file: file.into(),
            line: 7,
            column: 9,
        });
        let printed = if cfg!(feature = "small_path_src") {
            small
        } else {
            normal
        };
        assert_eq!(snapshot.er_report(), format!("error @ {printed}:7:9"));
        assert_eq!(
            snapshot.entries[0]
                .src_location
                .as_ref()
                .map(|location| location.file.as_ref()),
            Some(file)
        );
    }
}
