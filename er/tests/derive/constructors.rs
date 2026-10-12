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
pub fn empty_errors_with_unit() {
    fn unit() -> ErResult<(), EmptyUnitErr> {
        Err::<(), std::fmt::Error>(std::fmt::Error).er(())?;
        Ok(())
    }

    assert!(unit().unwrap_err().er_contains::<std::fmt::Error>());

    let error = std::fmt::Error.er_with::<EmptyUnitErr, _>(|_| ());
    assert!(error.er_contains::<std::fmt::Error>());
}

#[test]
pub fn construct_from_fields() {
    fn file(path: &std::path::Path) -> ErResult<(), FileConstructErr> {
        Err::<(), std::fmt::Error>(std::fmt::Error).er(|_| path)?;
        Ok(())
    }

    fn config(machine: &str, token: &str) -> ErResult<(), ConfigConstructErr> {
        Err::<(), std::fmt::Error>(std::fmt::Error).er(|_| (machine, token))?;
        Ok(())
    }

    let file = file(std::path::Path::new("config.toml")).unwrap_err();
    assert_eq!(file.top.path, PathBuf::from("config.toml"));
    assert!(file.er_contains::<std::fmt::Error>());

    let config = config("ComputerKatten", "secret").unwrap_err();
    assert_eq!(config.top.machine, "ComputerKatten");
    assert_eq!(config.top.token, "secret");

    fn chained(path: &std::path::Path) -> ErResult<(), ConfigConstructErr> {
        Err::<(), std::fmt::Error>(std::fmt::Error)
            .er::<FileConstructErr>(|_| path)
            .er(|_| ("machine", "token"))?;
        Ok(())
    }

    let chained = chained(std::path::Path::new("config.toml")).unwrap_err();
    assert!(chained.er_contains::<FileConstructErr>());

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
pub fn bail_from_fields() {
    fn empty() -> ErResult<(), EmptyUnitErr> {
        er_bail!(());
    }

    fn file(path: &std::path::Path, calls: &Cell<u8>) -> ErResult<(), FileConstructErr> {
        let make = |_| {
            calls.set(calls.get() + 1);
            path
        };

        er_bail!(make);
    }

    fn config(machine: &str, token: &str) -> Result<(), ErReport<ConfigConstructErr>> {
        er_bail!(|_| (machine, token));
    }

    fn job(path: &str) -> Result<(), ErTop<JobErr>> {
        er_bail!(|| JobErr::file(path, "bad"));
    }

    assert!(empty().unwrap_err().nodes.is_empty());

    let calls = Cell::new(0);
    let file = file(std::path::Path::new("config.toml"), &calls).unwrap_err();

    assert_eq!(file.top.path, PathBuf::from("config.toml"));
    assert!(file.nodes.is_empty());
    assert_eq!(calls.get(), 1);

    let config = config("ComputerKatten", "secret").unwrap_err();

    assert_eq!(config.tree.top.machine, "ComputerKatten");
    assert_eq!(config.tree.top.token, "secret");

    match job("config.toml").unwrap_err().tree.top {
        JobErr::File { path, msg } => {
            assert_eq!(path, Path::new("config.toml"));
            assert_eq!(msg, "bad");
        }
        _ => panic!("expected file error"),
    }
}
#[test]
pub fn boxed_error_can_be_used_or_wrapped() {
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
    let error = OptionalErr::new(Some(7u8));
    assert_eq!(error.to_string(), "OptionalErr { value: Some(7) }");

    let data = OptionalData::new(Some(7u8));
    assert_eq!(data.to_string(), "OptionalData { value: Some(7) }");
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
    assert_eq!(ManualErr::new(84).er_wrap().tree.top.0, 85);
    assert!(matches!(ManualData::pair(7), ManualData::Pair(7, 7)));

    let path = PathBuf::from("config.toml");
    let msg = String::from("couldn't read it");
    let token = Token("secret".into());
    let calls = Cell::new(0);
    let e = || {
        calls.set(calls.get() + 1);
        FileErr::new(&path, &msg, &token)
    };

    assert_eq!(Some(85).er::<FileErr>(e).er_top().unwrap(), 85);
    assert_eq!(calls.get(), 0);
    let error = None::<()>.er::<FileErr>(e).unwrap_err();
    assert_eq!(
        (&error.top.path, &error.top.msg, &error.top.token.0),
        (&path, &msg, &token.0)
    );

    assert_eq!(calls.get(), 1);

    assert!(
        matches!(JobErr::file(&path, "bad"), JobErr::File { path: p, msg } if p == path && msg == "bad")
    );
    assert!(
        matches!(JobErr::pair(&path, &token), JobErr::Pair(p, t) if p == path && t.0 == token.0)
    );
    assert!(matches!(JobErr::http_error(), JobErr::HTTPError));
    assert!(matches!(JobErr::r#type("bad"), JobErr::Type(s) if s == "bad"));
    assert!(matches!(JobErr::crate_(), JobErr::Crate));

    let missing: Option<()> = None;
    assert!(matches!(
        missing.er(JobErr::forbidden).unwrap_err().top,
        JobErr::Forbidden
    ));

    let owned = FileErr::new(path, msg, token);
    assert_eq!(owned.path, PathBuf::from("config.toml"));
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

    for tree in [
        convert(Err(RawValue(7)), "device").unwrap_err(),
        propagate(Err(RawValue(7)), "device").unwrap_err(),
    ] {
        assert_eq!(tree.top.path, "device");
        assert_eq!(tree.top.details.0, 92);
        assert!(tree.nodes.is_empty());
    }

    assert_eq!(convert(Ok(7), "device").unwrap_report(), 7);
    assert_eq!(propagate(Ok(8), "device").unwrap_report(), 8);
}

#[test]
pub fn previous_error_fields_and_aggregation() {
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
    assert!(error.er_contains::<ExactConstructErr>());

    let result: ErResult<(), FileConstructErr> =
        Err(error).er_with(|err| er_built(FileConstructErr::new(&err.path)));
    assert_eq!(result.unwrap_err().top.path, PathBuf::from("/dev/example"));

    fn variant(input: Result<(), FileConstructErr>) -> ErResult<(), JobErr> {
        input.er_with(|err| JobErr::file(&err.path, "bad"))?;
        Ok(())
    }

    fn value_variant(input: Result<(), PathBuf>) -> ErResult<(), JobErr> {
        input.er_val(|v| JobErr::file(v, "bad"))
    }

    let result = variant(Err(FileConstructErr::new("config.toml")));
    let tree = result.unwrap_err();
    assert!(matches!(&tree.top, JobErr::File { path, .. } if path == Path::new("config.toml")));
    assert!(tree.er_contains::<FileConstructErr>());

    let mapped = value_variant(Err(PathBuf::from("config.toml"))).unwrap_err();
    assert!(matches!(&mapped.top, JobErr::File { path, .. } if path == Path::new("config.toml")));
    assert!(mapped.nodes.is_empty());

    let result: ErResult<(), JobErr> = Err::<(), _>(tree).er_with_tree(|tree| {
        JobErr::file(&tree.er_find::<FileConstructErr>().unwrap().path, "bad")
    });
    assert_eq!(result.unwrap_err().er_find_all::<JobErr>().count(), 2);

    fn gather(path: &str) -> ErResult<(), FileConstructErr> {
        er_all!(|_| path, [Err::<(), _>(std::fmt::Error)])?;
        Ok(())
    }

    let gathered = gather("batch.txt").unwrap_err();
    assert_eq!(gathered.top.path, PathBuf::from("batch.txt"));

    fn gather_unit() -> ErResult<(), EmptyUnitErr> {
        er_all!((), [Err::<(), _>(std::fmt::Error)])?;
        Ok(())
    }

    assert!(gather_unit().unwrap_err().er_contains::<std::fmt::Error>());
}

#[test]
pub fn failed_construction_is_lazy() {
    struct Counted<'a>(&'a Cell<u8>);
    impl From<Counted<'_>> for String {
        fn from(value: Counted<'_>) -> Self {
            value.0.set(value.0.get() + 1);
            "built".into()
        }
    }

    let closure_calls = Cell::new(0);
    let conversions = Cell::new(0);
    let ok: Result<(), std::fmt::Error> = Ok(());
    let result: ErResult<(), ConfigConstructErr> = ok.er(|_| {
        closure_calls.set(closure_calls.get() + 1);
        (Counted(&conversions), "token")
    });
    assert!(result.is_ok());
    assert_eq!((closure_calls.get(), conversions.get()), (0, 0));

    let failed: Result<(), std::fmt::Error> = Err(std::fmt::Error);
    let _: ErTree<ConfigConstructErr> = failed
        .er(|_| {
            closure_calls.set(closure_calls.get() + 1);
            (Counted(&conversions), "token")
        })
        .unwrap_err();
    assert_eq!((closure_calls.get(), conversions.get()), (1, 1));
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
            Boundary::Saved { diagnostics, tag } => {
                assert!(diagnostics.contains("stopped"));
                assert_eq!(tag, 85);
            }
            Boundary::Source(_) => panic!("wrong variant"),
        }

        let source = Boundary::source(io::Error::other("source"));
        assert_eq!(source.er_snapshot().entries.len(), 2);
        assert_eq!(Data::new(value).to_string(), "Data(85)");
        assert_eq!(Root::new("task").0, "task");

        Ok(())
    }
}
