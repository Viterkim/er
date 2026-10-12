#![allow(non_upper_case_globals)]

use er::*;

const __er_try_errors: u8 = 7;
const __er_try_values: u8 = 8;

macro_rules! context {
    () => {
        __er_try_errors
    };
}
macro_rules! input {
    () => {
        Ok::<_, std::fmt::Error>(__er_try_values)
    };
}

#[derive(Er)]
pub struct Batch(pub u8);

fn failed() -> ErResult<((),), Batch> {
    er_try!(|_| context!(), [Err::<(), _>(std::fmt::Error)])
}

fn succeeded() -> ErResult<(u8,), Batch> {
    er_try!(|_| 0u8, [input!()])
}

fn main() {
    assert_eq!(failed().unwrap_err().top.0, 7);
    assert_eq!(succeeded().unwrap_report(), (8,));
}
