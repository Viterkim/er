use er::*;
use std::{
    cell::Cell,
    path::{Path, PathBuf},
};

#[derive(Er)]
pub struct EmptyUnitErr;

#[derive(Er)]
pub struct FileConstructErr {
    pub path: PathBuf,
}

#[derive(Er)]
pub struct ConfigConstructErr {
    pub machine: String,
    pub token: String,
}

#[derive(Er)]
pub struct ExactConstructErr {
    pub count: u8,
    pub label: String,
}

#[derive(Er)]
pub struct BoxConstructErr {
    pub source: Box<dyn std::error::Error + Send + Sync>,
}

#[derive(Er)]
pub struct PairTupleErr(pub (u8, u16));

#[test]
pub fn unit() {
    fn unit() -> ErResult<(), EmptyUnitErr> {
        Err::<(), std::fmt::Error>(std::fmt::Error).er(())?;
        Ok(())
    }

    assert!(unit().unwrap_err().er_contains::<std::fmt::Error>());
}

#[test]
pub fn fields() {
    fn file(path: &Path) -> ErResult<(), FileConstructErr> {
        Err::<(), std::fmt::Error>(std::fmt::Error).er(|_| path)?;
        Ok(())
    }

    let file = file(Path::new("config.toml")).unwrap_err();
    assert_eq!(file.top.path, PathBuf::from("config.toml"));

    fn tuple_field() -> ErResult<(), PairTupleErr> {
        None::<()>.er(|_| (85, 86))
    }

    assert_eq!(tuple_field().unwrap_err().top.0, (85, 86));

    let token = Token("secret".into());
    let error: ErTree<FileErr> = None::<()>
        .er(|_| (Path::new("config.toml"), "bad", &token))
        .unwrap_err();

    assert_eq!(error.top.token.0, "secret");
}

#[test]
pub fn bail() {
    fn file(path: &Path) -> ErResult<(), FileConstructErr> {
        er_bail!(|_| path);
    }

    fn job(path: &str) -> Result<(), ErTop<JobErr>> {
        er_bail!(|| JobErr::file(path, "bad"));
    }

    let file = file(Path::new("config.toml")).unwrap_err();
    assert_eq!(file.top.path, PathBuf::from("config.toml"));

    let job = job("config.toml").unwrap_err().tree.top;
    assert!(matches!(job, JobErr::File { msg, .. } if msg == "bad"));
}

#[test]
pub fn built_or_fields() {
    let original = BoxConstructErr::new(std::io::Error::other("original"));
    let used: ErResult<(), BoxConstructErr> =
        Err::<(), std::fmt::Error>(std::fmt::Error).er(|| original);
    let used = used.unwrap_err();
    assert!(used.top.source.is::<std::io::Error>());

    let original = BoxConstructErr::new(std::io::Error::other("original"));
    let wrapped: ErResult<(), BoxConstructErr> =
        Err::<(), std::fmt::Error>(std::fmt::Error).er(|_| original);
    let wrapped = wrapped.unwrap_err();
    assert!(wrapped.top.source.is::<BoxConstructErr>());

    let original = BoxConstructErr::new(std::io::Error::other("original"));
    let used: ErResult<(), BoxConstructErr> =
        Err::<(), _>(std::fmt::Error).er_with(|_| er_built(original));
    assert!(used.unwrap_err().top.source.is::<std::io::Error>());
}

#[derive(Er)]
pub struct OptionalErr<T> {
    #[er(exact)]
    pub value: Option<T>,
}

#[derive(ErFormat)]
pub struct OptionalData<T> {
    #[er(exact)]
    pub value: Option<T>,
}
#[test]
pub fn exact() {
    let _ = OptionalErr::new(Some(7u8));
    let _ = OptionalData::new(Some(7u8));
}

#[derive(Er)]
#[er(no_constructors, wrap)]
pub struct ManualErr(pub u8);
impl ManualErr {
    pub fn new(value: u8) -> Self {
        Self(value.saturating_add(1))
    }
}

#[derive(ErFormat)]
#[er(no_constructors)]
pub enum ManualData {
    Pair(u8, u8),
}
impl ManualData {
    pub fn pair(value: u8) -> Self {
        Self::Pair(value, value)
    }
}

#[derive(Debug)]
pub struct Token(pub String);
impl From<&Self> for Token {
    fn from(value: &Self) -> Self {
        Self(value.0.clone())
    }
}

#[derive(Er)]
pub struct FileErr {
    pub path: PathBuf,
    pub msg: String,
    pub token: Token,
}

#[derive(Er)]
pub enum JobErr {
    Forbidden,
    File { path: PathBuf, msg: String },
    Pair(PathBuf, Token),
    HTTPError,
    Type(String),
    Crate,
}
#[test]
pub fn constructors() {
    let _ = ManualErr::new(84).er_wrap();
    let _ = ManualData::pair(7);

    let path = PathBuf::from("config.toml");
    let token = Token("secret".into());
    let _ = FileErr::new(&path, "bad", &token);
    let _ = JobErr::pair(&path, &token);
    let _ = (
        JobErr::http_error(),
        JobErr::r#type("bad"),
        JobErr::crate_(),
    );

    let missing: Option<()> = None;
    let tree = missing.er(JobErr::forbidden).unwrap_err();
    assert!(matches!(tree.top, JobErr::Forbidden));
}

#[derive(Er)]
pub struct DeviceContextErr {
    pub code: u8,
    pub path: String,
}

pub struct RawValue(pub u8);

#[derive(ErFormat)]
pub struct Details(pub u16);
impl From<RawValue> for Details {
    fn from(value: RawValue) -> Self {
        Self(u16::from(value.0) + 85)
    }
}

#[derive(Er)]
pub struct ValueErr {
    pub path: String,
    pub details: Details,
}
#[test]
pub fn value_fields() {
    fn convert(input: Result<u8, RawValue>, path: &str) -> ErResult<u8, ValueErr> {
        input.er_val(|v| (path, v))
    }

    fn propagate(input: Result<u8, RawValue>, path: &str) -> ErResult<u8, ValueErr> {
        let value = input.er_val(|v| (path, v))?;
        Ok(value)
    }

    for convert in [convert, propagate] {
        let tree = convert(Err(RawValue(7)), "device").unwrap_err();
        assert_eq!(tree.top.details.0, 92);
        assert!(tree.nodes.is_empty());
    }
}

#[test]
pub fn previous_fields() {
    fn from_previous(path: &str) -> ErResult<(), DeviceContextErr> {
        let result: Result<(), ExactConstructErr> = Err(ExactConstructErr::new(85, "device"));
        result.er_with(|err| (err.count, path))?;
        Ok(())
    }

    let error = from_previous("/dev/example").unwrap_err();
    assert_eq!(
        (error.top.code, error.top.path.as_str()),
        (85, "/dev/example")
    );

    fn variant(input: Result<(), FileConstructErr>) -> ErResult<(), JobErr> {
        input.er_with(|err| JobErr::file(&err.path, "bad"))?;
        Ok(())
    }

    let tree = variant(Err(FileConstructErr::new("config.toml"))).unwrap_err();
    assert!(matches!(&tree.top, JobErr::File { path, .. } if path == Path::new("config.toml")));
}

#[test]
pub fn lazy_fields() {
    struct Counted<'a>(&'a Cell<u8>);
    impl From<Counted<'_>> for String {
        fn from(value: Counted<'_>) -> Self {
            value.0.set(value.0.get() + 1);
            "built".into()
        }
    }

    let conversions = Cell::new(0);
    let failed: Result<(), std::fmt::Error> = Err(std::fmt::Error);
    let _: ErTree<ConfigConstructErr> =
        failed.er(|_| (Counted(&conversions), "token")).unwrap_err();
    assert_eq!(conversions.get(), 1);
}

pub mod constants {
    use er::*;
    use std::io;

    #[expect(non_upper_case_globals, reason = "derive binding collisions")]
    pub mod names {
        pub const value: u8 = 85;
        pub const __er_f: u8 = 86;
        pub const __er_d: u8 = 87;
        pub const __er_0: u8 = 88;
        pub const __er_source: u8 = 89;
        pub const __er_value_0: u8 = 90;
        pub const __er_tree: u8 = 91;
        pub const __er_report: u8 = 92;
        pub const __er_extra: u8 = 93;
        pub const __er_saved_report: u8 = 94;
        pub const __er_emit: u8 = 95;
        pub const tree: u8 = 96;
        pub const error: u8 = 97;
        pub const report: u8 = 98;
        pub const top: u8 = 99;
        pub const wrapped: u8 = 100;
    }
    use names::*;

    #[derive(Er)]
    #[er(wrap(output = report, std_error))]
    pub struct Root(pub String);

    #[derive(Er)]
    pub enum Boundary {
        Saved {
            #[er(into_report_string)]
            diagnostics: String,
            tag: u8,
        },
        Source(#[er(source)] io::Error),
    }

    #[derive(ErFormat)]
    pub struct Data(pub u8);

    #[test]
    pub fn bindings() -> Result<(), BoxError> {
        let result: ErResult<(), Root> = Err::<(), _>(io::Error::other("stopped")).er(|_| "task");
        let wrapped_error = RootWrap::from(result.unwrap_err());

        let mut lines = Vec::new();
        wrapped_error.for_each_line(|l| lines.push(l.to_owned()))?;
        assert!(lines.iter().any(|line| line.contains("stopped")));

        let saved: Boundary = wrapped_error.er_into(|saved| (saved, value));
        match saved {
            Boundary::Saved { diagnostics, .. } => assert!(diagnostics.contains("stopped")),
            Boundary::Source(_) => panic!("wrong variant"),
        }

        let source = Boundary::source(io::Error::other("source"));
        assert_eq!(source.er_snapshot().entries.len(), 2);
        assert_eq!(Data::new(value).to_string(), "Data(85)");
        let _ = Root::new("task");

        Ok(())
    }
}
