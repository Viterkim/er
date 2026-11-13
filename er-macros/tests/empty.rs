use er_macros::{Er, ErFormat};

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

#[derive(ErFormat)]
#[er(format = "{name}")]
pub struct DefaultData<T> {
    pub name: String,
    #[er(default)]
    pub data: T,
}

#[test]
fn construction() {
    struct Payload;
    let data = DefaultData::new("supplied", Payload);
    let Payload = data.data;

    let _ =
        <EmptyUnit as er::internal::ErFromFields<(), er::internal::ErFields>>::er_from_fields(());
    let _ =
        <EmptyNamed as er::internal::ErFromFields<(), er::internal::ErFields>>::er_from_fields(());
    let _: ManualEmpty = ().into();
    let file = <FileErr as er::internal::ErFromFields<_, er::internal::ErFields>>::er_from_fields(
        "config.toml",
    );
    assert_eq!(file.0, "config.toml");
}
