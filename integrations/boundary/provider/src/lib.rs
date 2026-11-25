use er::*;

#[derive(Er)]
pub struct ReadErr;

#[derive(Er)]
#[er(
    format = "provider failed",
    debug = "provider failed, secret {secret}",
    display_alt = "{message}"
)]
pub struct Failure<T = ()> {
    pub message: String,
    #[er(default, skip)]
    pub data: T,
    #[er(default = String::from("secret sentinel"), censor)]
    pub secret: String,
}

pub fn read() -> Result<u8, (ReadErr, String)> {
    let result: ErResult<u8, ReadErr> = Err(std::fmt::Error).er(());

    result.er_split()
}

type Cause<E> = std::sync::Arc<E>;

#[derive(Er)]
pub enum OpenError {
    #[er(display = "access denied for {0}")]
    Denied(String),
    Failed {
        #[er(source(deref))]
        cause: Option<Cause<std::io::Error>>,
    },
}

#[derive(Er)]
#[er(no_constructors)]
pub struct ManualError {
    #[er(exact)]
    pub code: u16,
}
impl ManualError {
    pub fn new(code: u16) -> Self {
        Self { code }
    }
}

#[derive(Er)]
#[er(no_constructors)]
pub struct NoConstructors {
    pub code: u8,
}

#[derive(ErFormat)]
#[er(display = "revision {0}")]
pub struct Revision(pub u64);

#[derive(Er)]
#[er(display = Self::summary)]
pub struct FunctionError {
    pub revision: Revision,
}
impl FunctionError {
    pub fn summary(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "couldn't save {}", self.revision)
    }
}

#[derive(Er)]
#[er(format = "invalid {value:?} @ {location}")]
pub struct LocatedError {
    pub value: String,
    #[er(src_location)]
    pub location: &'static std::panic::Location<'static>,
}
