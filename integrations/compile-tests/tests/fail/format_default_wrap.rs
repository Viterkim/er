use compile_tests::BoundaryErr;
use er::*;

pub fn main() {
    let wrapped = compile_tests::BoundaryErrWrap::from(BoundaryErr.er());

    println!("{wrapped}");
    println!("{wrapped:?}");
}
