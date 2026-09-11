use er::*;

pub fn main() {
    let wrapped = compile_tests::BoundaryErrWrap::from(compile_tests::BoundaryErr.er());
    let _: Box<dyn core::error::Error> = Box::new(wrapped);
}
