use er::*;
use std::{cell::Cell, path::PathBuf};

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
    fn unit() -> Er<(), EmptyUnitErr> {
        Err::<(), std::fmt::Error>(std::fmt::Error).er(())?;
        Ok(())
    }

    assert!(unit().unwrap_err().er_contains::<std::fmt::Error>());
}

#[test]
pub fn construct_from_fields() {
    fn file(path: &std::path::Path) -> Er<(), FileConstructErr> {
        Err::<(), std::fmt::Error>(std::fmt::Error).er(|| path)?;
        Ok(())
    }

    fn config(machine: &str, token: &str) -> Er<(), ConfigConstructErr> {
        Err::<(), std::fmt::Error>(std::fmt::Error).er(|| (machine, token))?;
        Ok(())
    }

    let file = file(std::path::Path::new("config.toml")).unwrap_err();
    assert_eq!(file.top.path, PathBuf::from("config.toml"));
    assert!(file.er_contains::<std::fmt::Error>());

    let config = config("ComputerKatten", "secret").unwrap_err();
    assert_eq!(config.top.machine, "ComputerKatten");
    assert_eq!(config.top.token, "secret");

    fn chained(path: &std::path::Path) -> Er<(), ConfigConstructErr> {
        Err::<(), std::fmt::Error>(std::fmt::Error)
            .er::<FileConstructErr>(|| path)
            .er(|| ("machine", "token"))?;
        Ok(())
    }

    let chained = chained(std::path::Path::new("config.toml")).unwrap_err();
    assert!(chained.er_contains::<FileConstructErr>());

    fn tuple_field() -> Er<(), PairTupleErr> {
        None::<()>.er(|| (85, 86))
    }
    assert_eq!(tuple_field().unwrap_err().top.0, (85, 86));

    fn three(path: &std::path::Path, token: &Token) -> Er<(), FileErr> {
        None::<()>.er(|| (path, "bad", token))
    }
    let token = Token("secret".into());
    let error = three(std::path::Path::new("config.toml"), &token).unwrap_err();
    assert_eq!(error.top.token.0, "secret");
}

#[test]
pub fn boxed_error_can_be_used_or_wrapped() {
    let original = BoxConstructErr::new(std::io::Error::other("original"));
    let used: Er<(), BoxConstructErr> =
        ErContext::<ErBuilt>::er(Err::<(), std::fmt::Error>(std::fmt::Error), || original);
    let used = used.unwrap_err();
    assert!(used.top.source.is::<std::io::Error>());

    let original = BoxConstructErr::new(std::io::Error::other("original"));
    let wrapped: Er<(), BoxConstructErr> =
        ErContext::<ErFields>::er(Err::<(), std::fmt::Error>(std::fmt::Error), || original);
    let wrapped = wrapped.unwrap_err();
    assert!(wrapped.top.source.is::<BoxConstructErr>());
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
    let er = || {
        calls.set(calls.get() + 1);
        FileErr::new(&path, &msg, &token)
    };

    assert_eq!(Some(85).er::<FileErr>(er).er_top().unwrap(), 85);
    assert_eq!(calls.get(), 0);
    let error = None::<()>.er::<FileErr>(er).unwrap_err();
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

#[test]
pub fn previous_error_fields_and_aggregation() {
    fn from_previous(path: &str) -> Er<(), DeviceContextErr> {
        let result: Result<(), ExactConstructErr> = Err(ExactConstructErr::new(85, "device"));
        result.er_with(|old| (old.count, path))?;
        Ok(())
    }

    let error = from_previous("/dev/example").unwrap_err();
    assert_eq!(
        (error.top.code, error.top.path.as_str()),
        (85, "/dev/example")
    );
    assert!(error.er_contains::<ExactConstructErr>());

    fn gather(path: &str) -> Er<(), FileConstructErr> {
        er_all!(|| path, [Err::<(), _>(std::fmt::Error)])?;
        Ok(())
    }

    let gathered = gather("batch.txt").unwrap_err();
    assert_eq!(gathered.top.path, PathBuf::from("batch.txt"));

    fn gather_unit() -> Er<(), EmptyUnitErr> {
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
    let result: Er<(), ConfigConstructErr> = ok.er(|| {
        closure_calls.set(closure_calls.get() + 1);
        (Counted(&conversions), "token")
    });
    assert!(result.is_ok());
    assert_eq!((closure_calls.get(), conversions.get()), (0, 0));

    let failed: Result<(), std::fmt::Error> = Err(std::fmt::Error);
    let _: ErTree<ConfigConstructErr> = failed
        .er(|| {
            closure_calls.set(closure_calls.get() + 1);
            (Counted(&conversions), "token")
        })
        .unwrap_err();
    assert_eq!((closure_calls.get(), conversions.get()), (1, 1));
}
