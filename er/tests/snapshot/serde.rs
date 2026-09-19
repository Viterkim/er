use super::Message;
use er::*;

#[test]
pub fn round_trip() -> Result<(), serde_json::Error> {
    let source = Some(Box::new(Message::new("native source")));
    let root = Message {
        text: "HaandboldFuglen",
        source,
    };
    let child = Message::new("child")
        .er()
        .er::<Message>(|| Message::new("parent"));
    let tree = ErTree::new(root, [child]);
    let snapshot = tree.er_snapshot();

    let json = serde_json::to_string(&snapshot)?;
    let restored: ErSnapshot = serde_json::from_str(&json)?;

    assert_eq!(restored, snapshot);
    Ok(())
}

#[test]
pub fn locations() -> Result<(), serde_json::Error> {
    let without = r#"{"entries":[{"message":"HaandboldFuglen","kind":"Root","index":0,"parent":null,"depth":0,"is_last":true,"source_truncated":false}]}"#;
    let snapshot: ErSnapshot = serde_json::from_str(without)?;
    assert_eq!(snapshot.entries[0].message, "HaandboldFuglen");
    #[cfg(feature = "src_locations")]
    assert_eq!(snapshot.entries[0].src_location, None);

    let with = r#"{"entries":[{"message":"HaandboldFuglen","kind":"Root","index":0,"parent":null,"depth":0,"is_last":true,"source_truncated":false,"src_location":{"file":"src/lib.rs","line":7,"column":19}}]}"#;
    let snapshot: ErSnapshot = serde_json::from_str(with)?;
    assert_eq!(snapshot.entries[0].message, "HaandboldFuglen");
    #[cfg(feature = "src_locations")]
    assert_eq!(
        snapshot.entries[0].src_location.as_ref().map(|location| (
            location.file.as_str(),
            location.line,
            location.column
        )),
        Some(("src/lib.rs", 7, 19))
    );

    Ok(())
}
