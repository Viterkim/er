use er::*;

#[derive(Er)]
#[er(wrap(output = report))]
pub struct Report;

#[derive(Er)]
#[er(wrap(output = top))]
pub struct Top;

pub fn main() {
    let wrapped = compile_tests::BoundaryErrWrap::from(compile_tests::BoundaryErr.er());
    let _: Box<dyn core::error::Error> = Box::new(wrapped);

    let _: &dyn core::error::Error = &Report::new().er_wrap();
    let _: &dyn core::error::Error = &Top::new().er_wrap();

    let tree = Report::new().er();
    let _: &dyn core::error::Error = &tree.er_report();
    let _: &dyn core::error::Error = &tree.er_top();
    let _: &dyn core::error::Error = &Report::new().er().into_er_report();
    let _: &dyn core::error::Error = &Top::new().er().into_er_top();
}
