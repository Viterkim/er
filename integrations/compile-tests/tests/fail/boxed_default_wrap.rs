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

    let _: &dyn core::error::Error = &Report::new().er_wrap();
    let _: &dyn core::error::Error = &Top::new().er_wrap();

    let tree = ErTree::from(Report::new());
    let _: &dyn core::error::Error = &tree.er_report();
    let _: &dyn core::error::Error = &tree.er_top();
    let _: &dyn core::error::Error = &ErTree::from(Report::new()).into_er_report();
    let _: &dyn core::error::Error = &ErTree::from(Top::new()).into_er_top();
}
