#![allow(non_upper_case_globals)]

use er::*;
use std::{error::Error, fmt, io, marker::PhantomData, rc::Rc, sync::Arc};

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

pub type GenericBox<E> = Box<E>;
pub type GenericRc<E> = Rc<E>;
pub type GenericArc<E> = Arc<E>;

#[derive(Er)]
pub enum GenericAliasError<E: ?Sized> {
    Boxed(#[er(source(deref))] GenericBox<E>),
    Shared(#[er(source(deref))] GenericRc<E>),
    Atomic(#[er(source(deref))] GenericArc<E>),
    OptionalBox(#[er(source(deref))] Option<GenericBox<E>>),
    OptionalRc(#[er(source(deref))] Option<GenericRc<E>>),
    OptionalArc(#[er(source(deref))] Option<GenericArc<E>>),
    Wrapped(#[er(source(deref))] Box<Self>),
    OptionalWrapped(#[er(source(deref))] Option<Box<Self>>),
}

#[derive(Er)]
pub struct PointerError<P> {
    #[er(source(deref))]
    pub cause: P,
}

#[derive(Er)]
pub struct OptionalPointerError<P> {
    #[er(source(deref))]
    pub cause: Option<P>,
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
pub fn generic_aliases() {
    for error in [
        GenericAliasError::<io::Error>::boxed(Box::new(io::Error::other("boxed"))),
        GenericAliasError::shared(Rc::new(io::Error::other("shared"))),
        GenericAliasError::atomic(Arc::new(io::Error::other("atomic"))),
        GenericAliasError::optional_box(Some(Box::new(io::Error::other("optional box")))),
        GenericAliasError::optional_rc(Some(Rc::new(io::Error::other("optional rc")))),
        GenericAliasError::optional_arc(Some(Arc::new(io::Error::other("optional arc")))),
    ] {
        let expected: &io::Error = match &error {
            GenericAliasError::Boxed(cause) | GenericAliasError::OptionalBox(Some(cause)) => cause,
            GenericAliasError::Shared(cause) | GenericAliasError::OptionalRc(Some(cause)) => cause,
            GenericAliasError::Atomic(cause) | GenericAliasError::OptionalArc(Some(cause)) => cause,
            _ => panic!("expected a direct source"),
        };
        let source = error.source().unwrap().downcast_ref::<io::Error>().unwrap();
        assert!(std::ptr::eq(source, expected));
        assert!(ErTree::from(error).er_contains::<io::Error>());
    }

    let cause: Box<dyn Error + Send + Sync> = Box::new(io::Error::other("dynamic"));
    let error = GenericAliasError::<dyn Error + Send + Sync>::boxed(cause);
    assert!(error.source().is_some_and(|cause| cause.is::<io::Error>()));

    let error = GenericAliasError::<io::Error>::Wrapped(Box::new(GenericAliasError::boxed(
        Box::new(io::Error::other("wrapped")),
    )));
    assert!(ErTree::from(error).er_contains::<io::Error>());

    let error = GenericAliasError::<io::Error>::OptionalWrapped(Some(Box::new(
        GenericAliasError::boxed(Box::new(io::Error::other("optional wrapped"))),
    )));
    assert!(ErTree::from(error).er_contains::<io::Error>());

    for error in [
        GenericAliasError::<io::Error>::OptionalBox(None),
        GenericAliasError::OptionalRc(None),
        GenericAliasError::OptionalArc(None),
        GenericAliasError::OptionalWrapped(None),
    ] {
        assert!(error.source().is_none());
    }

    let cause = Arc::new(io::Error::other("pointer"));
    let error = PointerError::<Arc<io::Error>>::new(cause.clone());
    let source = error.source().unwrap().downcast_ref::<io::Error>().unwrap();
    assert!(std::ptr::eq(source, &*cause));

    let error = OptionalPointerError::<Arc<io::Error>>::new(Some(cause.clone()));
    let source = error.source().unwrap().downcast_ref::<io::Error>().unwrap();
    assert!(std::ptr::eq(source, &*cause));
    assert!(
        OptionalPointerError::<Arc<io::Error>>::new(None)
            .source()
            .is_none()
    );
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

#[test]
pub fn custom_trait() {
    trait DriverFailure<T = ()>: Error {}
    impl<T> DriverFailure<T> for io::Error {}

    #[derive(Er)]
    #[er(format = "driver failed")]
    struct Driver {
        #[er(source)]
        cause: Box<dyn DriverFailure + Send + Sync>,
    }

    #[derive(Er)]
    #[er(format = "driver failed")]
    struct OptionalDriver {
        #[er(source)]
        cause: Option<Arc<dyn DriverFailure + Send + Sync>>,
    }

    #[derive(Er)]
    #[er(format = "driver failed")]
    struct GenericDriver<T> {
        #[er(source)]
        cause: Box<dyn DriverFailure<T> + Send + Sync>,
    }

    let error = GenericDriver::<u8>::new(Box::new(io::Error::other("generic driver sentinel"))
        as Box<dyn DriverFailure<u8> + Send + Sync>);
    assert!(error.source().unwrap().is::<io::Error>());

    let cause: Box<dyn DriverFailure + Send + Sync> = Box::new(io::Error::other("driver sentinel"));
    let error = Driver::new(cause);
    assert!(error.source().unwrap().is::<io::Error>());

    let cause = Arc::new(io::Error::other("shared driver sentinel"));
    let shared: Arc<dyn DriverFailure + Send + Sync> = cause.clone();
    let error = OptionalDriver::new(Some(shared));
    assert!(std::ptr::eq(
        error.source().unwrap().downcast_ref::<io::Error>().unwrap(),
        &*cause
    ));
    assert!(OptionalDriver::new(None).source().is_none());
}
