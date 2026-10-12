#![allow(non_upper_case_globals)]

use er::*;
use std::{error::Error, fmt, io, marker::PhantomData, sync::Arc};

#[derive(Er)]
pub struct FileError {
    pub path: String,
    #[er(source)]
    pub cause: io::Error,
}

#[derive(Er)]
#[er(wrap(output = report))]
pub enum CallError<E> {
    Closed,
    Failed(#[er(source)] E),
    Wrapped(#[er(source)] Box<Self>),
    Shared {
        request: u32,
        #[er(source)]
        cause: Arc<E>,
    },
}

#[derive(Er)]
pub struct OptionalError {
    #[er(source, skip)]
    pub cause: ::core::option::r#Option<Box<dyn Error + Send + Sync>>,
}

#[derive(Er)]
pub struct BorrowedError<'a, const __er_source: usize> {
    pub request: &'a str,
    #[er(source)]
    pub cause: &'a io::Error,
}

#[derive(Er)]
pub struct MutableError<'a, E: ?Sized> {
    #[er(source)]
    pub cause: &'a mut E,
}

pub type Cause = Box<io::Error>;
pub type SharedCause = Arc<io::Error>;
pub type BorrowedCause = &'static MethodSource;

#[derive(Er)]
pub enum AliasedError {
    Boxed(#[er(source)] BoxError),
    File(#[er(source)] Cause),
    Shared(#[er(source)] SharedCause),
    Borrowed(#[er(source)] BorrowedCause),
}

#[derive(Er)]
pub struct BoxedError<E: ?Sized> {
    #[er(source)]
    pub cause: ::std::boxed::r#Box<E>,
}

#[derive(Er)]
pub struct MethodSource;
impl MethodSource {
    pub fn __er_as_source(self: &&Self) -> &'static dyn Error {
        panic!("source conversion called an inherent method")
    }
}

#[derive(Er)]
pub struct MethodError {
    #[er(source)]
    pub cause: MethodSource,
}

pub trait __ErAsDynError {}
pub trait __ErAsSource {}

pub struct SourceTag;
impl __ErAsDynError for SourceTag {}
impl __ErAsSource for SourceTag {}

#[derive(Er)]
pub struct BoundError<T: __ErAsDynError>
where
    T: __ErAsSource,
{
    #[er(source)]
    pub cause: io::Error,
    #[er(skip)]
    pub tag: PhantomData<T>,
}

pub trait LineBound<const N: u32> {}
impl<const N: u32> LineBound<N> for () {}

pub struct LineSource<T, const N: u32>(pub PhantomData<T>);
impl<T: LineBound<N>, const N: u32> fmt::Display for LineSource<T, N> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{N}")
    }
}
impl<T: LineBound<N>, const N: u32> fmt::Debug for LineSource<T, N> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, formatter)
    }
}
impl<T: LineBound<N> + 'static, const N: u32> Error for LineSource<T, N> {}

macro_rules! line_source {
    ($t:ty) => { LineSource<$t, { line!() }> };
}

#[derive(Er)]
pub enum TypeMacroError<T> {
    First(#[er(source)] line_source!(T)),
    Second(#[er(source)] line_source!(T)),
}

#[test]
pub fn type_macros() {
    let first = TypeMacroError::<()>::First(LineSource(PhantomData));
    let second = TypeMacroError::<()>::Second(LineSource(PhantomData));

    assert_ne!(
        first.source().unwrap().to_string(),
        second.source().unwrap().to_string()
    );
}

#[test]
pub fn sources() {
    for error in [
        &MethodError::new(MethodSource) as &dyn Error,
        &AliasedError::borrowed(&MethodSource),
    ] {
        assert!(
            error
                .source()
                .is_some_and(|cause| cause.is::<MethodSource>())
        );
    }

    let error = FileError::new("katten.txt", io::Error::other("file"));
    assert!(error.source().is_some_and(|cause| cause.is::<io::Error>()));

    let tree = ErTree::from(error);
    assert_eq!(tree.er_find::<io::Error>().unwrap().to_string(), "file");
    assert!(tree.er_report().to_string().contains("file"));

    let error = BoundError::<SourceTag>::new(io::Error::other("bound"), PhantomData);
    assert!(ErTree::from(error).er_contains::<io::Error>());

    for error in [
        CallError::failed(io::Error::other("call")),
        CallError::shared(85, Arc::new(io::Error::other("call"))),
    ] {
        assert!(error.source().is_some_and(|cause| cause.is::<io::Error>()));
    }
    assert!(CallError::<io::Error>::closed().source().is_none());

    let error = CallError::wrapped(Box::new(CallError::failed(io::Error::other("wrapped"))));
    assert!(error.er_wrap().er_find::<io::Error>().is_some());
    assert!(CallError::failed(()).to_string().contains("()"));

    let request = String::from("katten");
    let cause = io::Error::other("borrowed");
    let error = BorrowedError::<85>::new(&request, &cause);
    assert!(error.source().is_some_and(|cause| cause.is::<io::Error>()));

    let mut cause = io::Error::other("mutable");
    let error = MutableError::new(&mut cause);
    assert!(error.source().is_some_and(|cause| cause.is::<io::Error>()));

    let cause: &mut dyn Error = &mut cause;
    let error = MutableError::new(cause);
    assert!(error.source().is_some_and(|cause| cause.is::<io::Error>()));

    for error in [
        AliasedError::boxed(io::Error::other("boxed")),
        AliasedError::file(Box::new(io::Error::other("file"))),
        AliasedError::shared(Arc::new(io::Error::other("shared"))),
    ] {
        assert!(error.source().is_some_and(|cause| cause.is::<io::Error>()));
        assert!(ErTree::from(error).er_find::<io::Error>().is_some());
    }
}

#[test]
pub fn boxed() {
    let errors: Vec<Box<dyn Error>> = vec![
        Box::new(BoxedError::<io::Error>::new(Box::new(io::Error::other(
            "sized",
        )))),
        Box::new(BoxedError::<dyn Error>::new(io::Error::other("dyn"))),
        Box::new(BoxedError::<dyn Error + Send>::new(
            Box::new(io::Error::other("send")) as Box<dyn Error + Send>,
        )),
        Box::new(BoxedError::<dyn Error + Sync>::new(
            Box::new(io::Error::other("sync")) as Box<dyn Error + Sync>,
        )),
        Box::new(BoxedError::<dyn Error + Send + Sync>::new(
            io::Error::other("both"),
        )),
    ];

    for error in errors {
        assert!(error.source().is_some_and(|cause| cause.is::<io::Error>()));
    }
}

#[test]
pub fn optional() {
    let error = OptionalError::new(None::<Box<dyn Error + Send + Sync>>);
    assert!(error.source().is_none());

    let error =
        OptionalError::new(Box::new(io::Error::other("boxed")) as Box<dyn Error + Send + Sync>);
    assert!(error.source().is_some_and(|cause| cause.is::<io::Error>()));
    assert!(!error.to_string().contains("boxed"));
}
