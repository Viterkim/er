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
fn standalone_construction() {
    let data: DefaultData<()> = (("saved", ()),).into();
    assert_eq!(data.name, "saved");
    let () = data.data;

    struct Payload;
    let data = DefaultData::new("supplied", Payload);
    let Payload = data.data;

    let _: EmptyUnit = ().into();
    let _: EmptyNamed = ().into();
    let _: ManualEmpty = ().into();
    let file: FileErr = ("config.toml",).into();
    assert_eq!(file.0, "config.toml");
}

#[derive(Er)]
#[er(format = "saved")]
pub struct Saved<T = ()> {
    #[er(into_report_string)]
    pub diagnostics: String,
    #[er(default)]
    pub data: T,
}

#[test]
fn standalone_saving() {
    let saved: Saved = ((String::from("saved diagnostics"),), [(); 4]).into();
    assert_eq!(saved.diagnostics, "saved diagnostics");
    let () = saved.data;
}
