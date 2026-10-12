use er::*;
use std::{error::Error, io, rc::Rc, sync::Arc};

type Cause<E> = Box<E>;
type SharedCause<E> = Rc<E>;
type AtomicCause<E> = Arc<E>;

#[derive(Er)]
pub enum Boundary<E: ?Sized> {
    Boxed(#[er(source(deref))] Cause<E>),
    Shared(#[er(source(deref))] SharedCause<E>),
    Atomic(#[er(source(deref))] AtomicCause<E>),
    OptionalBox(#[er(source(deref))] Option<Cause<E>>),
    OptionalRc(#[er(source(deref))] Option<SharedCause<E>>),
    OptionalArc(#[er(source(deref))] Option<AtomicCause<E>>),
}

#[derive(Er)]
pub struct Pointer<P> {
    #[er(source(deref))]
    pub cause: P,
}

#[derive(Er)]
pub struct OptionalPointer<P> {
    #[er(source(deref))]
    pub cause: Option<P>,
}

fn main() {
    let error = Boundary::<io::Error>::shared(Rc::new(io::Error::other("shared")));
    assert!(error.source().unwrap().is::<io::Error>());

    let cause: Box<dyn Error> = Box::new(io::Error::other("dynamic"));
    let error = Boundary::<dyn Error>::boxed(cause);
    assert!(error.source().unwrap().is::<io::Error>());

    let error = Pointer::<Arc<io::Error>>::new(Arc::new(io::Error::other("pointer")));
    assert!(error.source().unwrap().is::<io::Error>());

    for error in [
        Boundary::<io::Error>::OptionalBox(Some(Box::new(io::Error::other("box")))),
        Boundary::OptionalRc(Some(Rc::new(io::Error::other("rc")))),
        Boundary::OptionalArc(Some(Arc::new(io::Error::other("arc")))),
    ] {
        assert!(error.source().unwrap().is::<io::Error>());
    }

    for error in [
        Boundary::<io::Error>::OptionalBox(None),
        Boundary::OptionalRc(None),
        Boundary::OptionalArc(None),
    ] {
        assert!(error.source().is_none());
    }

    let error = OptionalPointer::<Arc<io::Error>>::new(Some(Arc::new(io::Error::other("pointer"))));
    assert!(error.source().unwrap().is::<io::Error>());
    assert!(OptionalPointer::<Arc<io::Error>>::new(None).source().is_none());
}
