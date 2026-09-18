use er_macros::Er;

#[derive(Er)]
pub struct EmptyUnit;

#[derive(Er)]
pub struct EmptyNamed {}

#[derive(Er)]
pub struct FileErr(pub String);

#[derive(Er)]
#[er(no_constructors)]
pub struct ManualEmpty;

impl From<()> for ManualEmpty {
    fn from(_: ()) -> Self {
        Self
    }
}

#[test]
fn standalone_construction() {
    let _: EmptyUnit = ().into();
    let _: EmptyNamed = ().into();
    let _: ManualEmpty = ().into();
    let file: FileErr = ("config.toml",).into();
    assert_eq!(file.0, "config.toml");
}
