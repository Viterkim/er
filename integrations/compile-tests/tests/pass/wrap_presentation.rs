use compile_tests::{BoundaryErr, BoundaryErrWrap};
use er::*;

pub fn main() {
    // Wrap from another crate, its tree is still public.
    let wrapped = BoundaryErrWrap::from(BoundaryErr.er());

    println!("{}", wrapped.er_top());
    println!("{}", wrapped.er_report());

    let recovered = wrapped.tree;
    assert!(recovered.er_find::<BoundaryErr>().is_some());
}
