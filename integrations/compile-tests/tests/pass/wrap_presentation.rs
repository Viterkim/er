use compile_tests::{BoundaryErr, BoundaryErrWrap};
use er::*;

#[derive(Er)]
#[er(wrap(output = top, std_error))]
pub struct ResponseErr;

impl compile_tests::BoundaryError for ResponseErrWrap {
    fn status_code(&self) -> u16 {
        400
    }
}

#[derive(Er)]
#[er(wrap(name = __ErNodeRoot, output = report))]
pub struct LocalErr(pub std::rc::Rc<u8>);

pub fn main() {
    let local = LocalErr::new(std::rc::Rc::new(7)).er_wrap();
    assert!(local.opaque_err().to_string().contains("LocalErr(7)"));

    // Wrap from another crate, its tree is still public.
    let wrapped = BoundaryErrWrap::from(ErTree::from(BoundaryErr));

    println!("{}", wrapped.er_top());
    println!("{}", wrapped.er_report());

    let recovered = wrapped.tree;
    assert!(recovered.er_find::<BoundaryErr>().is_some());

    let wrapped = ResponseErrWrap::from(ErTree::new(ResponseErr, [core::fmt::Error]));
    let foreign: &dyn compile_tests::BoundaryError = &wrapped;
    assert_eq!(foreign.status_code(), 400);
    assert_eq!(foreign.to_string(), "ResponseErr");
    assert!(foreign.source().is_none());
    let result = Err::<(), _>(wrapped).er_wrap(BoundaryErr::new);
    assert!(result.is_err_and(|tree| tree.er_contains::<core::fmt::Error>()));
}
