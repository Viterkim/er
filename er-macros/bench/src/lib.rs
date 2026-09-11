use er::*;
use std::{error::Error, fmt};

#[derive(Debug)]
pub struct ReadEr;
impl fmt::Display for ReadEr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("read port")
    }
}
impl Error for ReadEr {}
pub fn read_port(input: &str) -> Er<u16, ReadEr> {
    input.parse().er(|| ReadEr)
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
    pub struct SecretEr<T> {
        pub value: T,
        pub width: usize,
        #[er(censor)]
        pub secret: String,
    }

    #[derive(Er)]
    #[er(wrap)]
    pub struct Wrapped;
}
