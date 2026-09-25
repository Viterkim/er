use er::*;
use std::{error::Error, fmt};

#[derive(Debug)]
pub struct ReadErr;
impl fmt::Display for ReadErr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("read port")
    }
}
impl Error for ReadErr {}
pub fn read_port(input: &str) -> ErResult<u16, ReadErr> {
    input.parse().er(|| ReadErr)
}

#[cfg(feature = "derive")]
pub mod derived {
    use er::Er;
    use std::marker::PhantomData;

    include!(concat!(env!("OUT_DIR"), "/errors.rs"));

    pub trait HasValue {
        type Value;
    }
    #[derive(Er)]
    pub struct Projected<T> {
        #[er(skip)]
        pub marker: PhantomData<T>,
        pub value: <Self as HasValue>::Value,
    }
    impl<T> HasValue for Projected<T> {
        type Value = T;
    }

    #[derive(Er)]
    #[er(format = "{value} / {value:?} / {value:x}; {secret:>width$}", wrap(output = report))]
    pub struct SecretErr<T> {
        pub value: T,
        pub width: usize,
        #[er(censor)]
        pub secret: String,
    }

    #[derive(Er)]
    #[er(wrap)]
    pub struct Wrapped;
}
