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

#[allow(non_camel_case_types)]
pub trait __er_alloc {}
impl __er_alloc for u8 {}

#[derive(Er)]
#[er(wrap(output = top, std_error))]
pub struct GenericResponse<T: __er_alloc>(pub T);

pub fn main() {
    let wrapped =
        GenericResponseWrap::from(ErTree::new(GenericResponse::new(7u8), [core::fmt::Error]));
    assert_eq!(wrapped.er_top_string(), "GenericResponse(7)");
    assert_eq!(wrapped.er_snapshot().entries.len(), 2);

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
    assert_eq!(wrapped.er_top_string(), "ResponseErr");
    assert!(wrapped.er_report_string().contains("formatting"));
    assert_eq!(wrapped.er_snapshot().entries.len(), 2);
    let result = Err::<(), _>(wrapped).er_wrap(BoundaryErr::new);
    assert!(result.is_err_and(|tree| tree.er_contains::<core::fmt::Error>()));
}
