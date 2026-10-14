pub mod frozen;

#[derive(old_er::Er)]
pub struct ReadErr;

#[derive(er_macros::Er)]
#[er(
    format = "provider failed",
    debug = "provider failed, secret {secret}",
    display_alt = "{diagnostics}"
)]
pub struct Failure<T = ()> {
    #[er(into_top, source)]
    pub kind: ReadErr,
    #[er(into_report_string)]
    pub diagnostics: String,
    #[er(into_report_string(single_line))]
    pub single_line: String,
    #[er(default, skip)]
    pub data: T,
    #[er(default = String::from("secret sentinel"), censor)]
    pub secret: String,
}

pub fn read() -> Result<u8, Failure> {
    use old_er::*;

    let result: ErResult<u8, ReadErr> = Err(std::fmt::Error).er(());
    match result {
        Ok(value) => Ok(value),
        Err(tree) => {
            let diagnostics = tree.er_report_string();
            let single_line = tree.er_report().single_line().to_string();
            Err(Failure::new(
                tree.top,
                diagnostics,
                single_line,
                (),
                "secret sentinel",
            ))
        }
    }
}

type Cause<E> = std::sync::Arc<E>;

#[derive(er_macros::Er)]
pub enum OpenError {
    #[er(display = "access denied for {0}")]
    Denied(String),
    Failed {
        #[er(source(deref))]
        cause: Option<Cause<std::io::Error>>,
    },
}

#[derive(er_macros::Er)]
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

#[derive(er_macros::Er)]
#[er(no_constructors)]
pub struct NoConstructors {
    pub code: u8,
}

#[derive(er_macros::ErFormat)]
#[er(display = "revision {0}")]
pub struct Revision(pub u64);

#[derive(er_macros::Er)]
#[er(display = Self::summary)]
pub struct FunctionError {
    pub revision: Revision,
}
impl FunctionError {
    pub fn summary(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "couldn't save {}", self.revision)
    }
}

#[derive(er_macros::Er)]
#[er(format = "invalid {value:?} @ {location}")]
pub struct LocatedError {
    pub value: String,
    #[er(src_location)]
    pub location: &'static std::panic::Location<'static>,
}
