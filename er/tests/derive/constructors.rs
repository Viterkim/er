use er::*;
use std::{cell::Cell, path::PathBuf};

#[derive(Er)]
pub struct OptionalEr<T> {
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
    let error = OptionalEr::new(Some(7u8));
    assert_eq!(error.to_string(), "OptionalEr { value: Some(7) }");
    let data = OptionalData::new(Some(7u8));
    assert_eq!(data.to_string(), "OptionalData { value: Some(7) }");
}

#[derive(Er)]
#[er(no_constructors, wrap)]
pub struct ManualEr(pub u8);
impl ManualEr {
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
pub struct FileEr {
    pub path: PathBuf,
    pub msg: String,
    pub token: Token,
}

#[derive(Er)]
pub enum JobEr {
    Forbidden,
    File { path: PathBuf, msg: String },
    Pair(PathBuf, Token),
    HTTPError,
    Type(String),
    Crate,
}
#[test]
pub fn constructors() {
    assert_eq!(ManualEr::new(84).er_wrap().tree.top.0, 85);
    assert!(matches!(ManualData::pair(7), ManualData::Pair(7, 7)));

    let path = PathBuf::from("config.toml");
    let msg = String::from("couldn't read it");
    let token = Token("secret".into());
    let calls = Cell::new(0);
    let er = || {
        calls.set(calls.get() + 1);
        FileEr::new(&path, &msg, &token)
    };

    assert_eq!(Some(85).er(er).er_top().unwrap(), 85);
    assert_eq!(calls.get(), 0);
    let error = None::<()>.er(er).unwrap_err();
    assert_eq!(
        (&error.top.path, &error.top.msg, &error.top.token.0),
        (&path, &msg, &token.0)
    );

    assert_eq!(calls.get(), 1);

    assert!(
        matches!(JobEr::file(&path, "bad"), JobEr::File { path: p, msg } if p == path && msg == "bad")
    );
    assert!(matches!(JobEr::pair(&path, &token), JobEr::Pair(p, t) if p == path && t.0 == token.0));
    assert!(matches!(JobEr::http_error(), JobEr::HTTPError));
    assert!(matches!(JobEr::r#type("bad"), JobEr::Type(s) if s == "bad"));
    assert!(matches!(JobEr::crate_(), JobEr::Crate));

    let missing: Option<()> = None;
    assert!(matches!(
        missing.er(JobEr::forbidden).unwrap_err().top,
        JobEr::Forbidden
    ));

    let owned = FileEr::new(path, msg, token);
    assert_eq!(owned.path, PathBuf::from("config.toml"));
}
