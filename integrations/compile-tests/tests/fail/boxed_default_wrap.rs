use er::*;

#[derive(Er)]
#[er(wrap(output = report))]
pub struct Report;

#[derive(Er)]
#[er(wrap(output = top))]
pub struct Top;

pub fn main() {
    let wrapped = compile_tests::BoundaryErrWrap::from(ErTree::from(compile_tests::BoundaryErr));
    let _: Box<dyn core::error::Error> = Box::new(wrapped);

    let _: &dyn core::error::Error = &ReportWrap::from(Report::new());
    let _: &dyn core::error::Error = &TopWrap::from(Top::new());
}
