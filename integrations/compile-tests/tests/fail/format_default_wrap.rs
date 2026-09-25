use compile_tests::BoundaryErr;
use er::*;

pub fn main() {
    let wrapped = compile_tests::BoundaryErrWrap::from(ErTree::from(BoundaryErr));

    println!("{wrapped}");
    println!("{wrapped:?}");
}
