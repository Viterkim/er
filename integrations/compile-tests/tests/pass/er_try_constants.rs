#![allow(non_upper_case_globals)]

use er::*;

const __er_value_0: u8 = 7;
const __er_value_1: u8 = 8;
const __er_errors: u8 = 9;
const __er_output: u8 = 10;
const values: u8 = 11;
const errors: u8 = 12;

macro_rules! input {
    () => {
        Ok::<_, std::fmt::Error>(__er_value_0)
    };
}
macro_rules! context {
    () => {
        __er_value_1 + __er_errors + __er_output + values + errors
    };
}

#[derive(Er)]
pub struct Batch(pub u8);

fn main() {
    let result: ErResult<(u8, ()), Batch> =
        er_try!(|_| context!(), [input!(), Err::<(), _>(std::fmt::Error)]);
    assert_eq!(result.unwrap_err().top.0, 50);

    let result: ErResult<(u8, u8), Batch> = er_try!(|_| context!(), [input!(), input!()]);
    assert_eq!(result.unwrap_report(), (7, 7));
}
