#[cfg(target_has_atomic = "ptr")]
use alloc::sync::Arc;
use alloc::{boxed::Box, rc::Rc};
use core::error::Error;

pub trait ErAsDynError {
    fn __er_as_dyn(&self) -> &(dyn Error + 'static);
}
impl<T: Error + 'static> ErAsDynError for T {
    fn __er_as_dyn(&self) -> &(dyn Error + 'static) {
        self
    }
}
impl ErAsDynError for dyn Error + 'static {
    fn __er_as_dyn(&self) -> &(dyn Error + 'static) {
        self
    }
}
impl ErAsDynError for dyn Error + Send + 'static {
    fn __er_as_dyn(&self) -> &(dyn Error + 'static) {
        self
    }
}
impl ErAsDynError for dyn Error + Sync + 'static {
    fn __er_as_dyn(&self) -> &(dyn Error + 'static) {
        self
    }
}
impl ErAsDynError for dyn Error + Send + Sync + 'static {
    fn __er_as_dyn(&self) -> &(dyn Error + 'static) {
        self
    }
}

// Dispatch on our own receiver, so an error's inherent methods can't win.
pub struct ErSourceRef<'a, T: ?Sized>(pub &'a T);

// What dispatch hands back. The derive unwraps it with a full path, so a same-named method from someone
// else's trait is a type error instead of quietly becoming the source.
pub struct ErSourceFound<'a>(pub Option<&'a (dyn Error + 'static)>);
impl ErSourceRef<'_, ()> {
    pub fn __er_finish(found: ErSourceFound<'_>) -> Option<&(dyn Error + 'static)> {
        found.0
    }
}

pub trait ErAsSource<'a> {
    fn __er_as_source(self) -> ErSourceFound<'a>;
}
impl<'a, T: ErAsDynError + ?Sized> ErAsSource<'a> for &&ErSourceRef<'a, T> {
    fn __er_as_source(self) -> ErSourceFound<'a> {
        ErSourceFound(Some(ErAsDynError::__er_as_dyn(self.0)))
    }
}
impl<'a, T: ErAsDynError + ?Sized> ErAsSource<'a> for &ErSourceRef<'a, &T> {
    fn __er_as_source(self) -> ErSourceFound<'a> {
        ErSourceFound(Some(ErAsDynError::__er_as_dyn(&**self.0)))
    }
}
impl<'a, T: ErAsDynError + ?Sized> ErAsSource<'a> for &ErSourceRef<'a, &mut T> {
    fn __er_as_source(self) -> ErSourceFound<'a> {
        ErSourceFound(Some(ErAsDynError::__er_as_dyn(&**self.0)))
    }
}
impl<'a, T: ErAsDynError + ?Sized> ErAsSource<'a> for &ErSourceRef<'a, Box<T>> {
    fn __er_as_source(self) -> ErSourceFound<'a> {
        ErSourceFound(Some(ErAsDynError::__er_as_dyn(&**self.0)))
    }
}
impl<'a, T: ErAsDynError + ?Sized> ErAsSource<'a> for &ErSourceRef<'a, Rc<T>> {
    fn __er_as_source(self) -> ErSourceFound<'a> {
        ErSourceFound(Some(ErAsDynError::__er_as_dyn(&**self.0)))
    }
}
#[cfg(target_has_atomic = "ptr")]
impl<'a, T: ErAsDynError + ?Sized> ErAsSource<'a> for &ErSourceRef<'a, Arc<T>> {
    fn __er_as_source(self) -> ErSourceFound<'a> {
        ErSourceFound(Some(ErAsDynError::__er_as_dyn(&**self.0)))
    }
}
// Unwrap an aliased Option before resolving its source, so pointer dispatch still gets a turn.
pub trait ErSourceOption<'a> {
    type Source: ?Sized + 'a;

    fn __er_source_option(self) -> Option<ErSourceRef<'a, Self::Source>>;
}
impl<'a, T: ?Sized + 'a> ErSourceOption<'a> for &&ErSourceRef<'a, T> {
    type Source = T;

    fn __er_source_option(self) -> Option<ErSourceRef<'a, T>> {
        Some(ErSourceRef(self.0))
    }
}
impl<'a, T: 'a> ErSourceOption<'a> for &ErSourceRef<'a, Option<T>> {
    type Source = T;

    fn __er_source_option(self) -> Option<ErSourceRef<'a, T>> {
        self.0.as_ref().map(ErSourceRef)
    }
}
