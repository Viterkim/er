#[cfg(all(er_unsync, target_has_atomic = "ptr"))]
use crate::ErSavedArc;
use crate::{
    ErDefaults, ErReport, ErSavedShared, ErShared, ErSnapshot, ErValFields, IntoErSnapshot,
};
use alloc::rc::Rc;
#[cfg(er_unsync)]
use alloc::rc::Rc as Shared;
#[cfg(target_has_atomic = "ptr")]
use alloc::sync::Arc;
#[cfg(not(er_unsync))]
use alloc::sync::Arc as Shared;
use core::{fmt, ops::Deref};

impl<T> ErShared<T> {
    /// Share the value without copying it.
    pub fn new(value: T) -> Self {
        Self {
            value: Shared::new(value),
        }
    }
}
impl<T: ?Sized> ErShared<T> {
    /// Whether both handles point to the same allocation.
    pub fn ptr_eq(this: &Self, other: &Self) -> bool {
        Shared::ptr_eq(&this.value, &other.value)
    }
}
impl<T> From<T> for ErShared<T> {
    fn from(value: T) -> Self {
        Self::new(value)
    }
}
impl<T: Clone> From<&T> for ErShared<T> {
    fn from(value: &T) -> Self {
        Self::new(value.clone())
    }
}
impl<T: ?Sized> From<Shared<T>> for ErShared<T> {
    fn from(value: Shared<T>) -> Self {
        Self { value }
    }
}
impl<T: ?Sized> From<&Shared<T>> for ErShared<T> {
    fn from(value: &Shared<T>) -> Self {
        Self::from(Shared::clone(value))
    }
}
impl<T: ?Sized> From<&Self> for ErShared<T> {
    fn from(value: &Self) -> Self {
        value.clone()
    }
}
impl<T: ?Sized> Clone for ErShared<T> {
    fn clone(&self) -> Self {
        Self {
            value: Shared::clone(&self.value),
        }
    }
}
impl<T: ?Sized> Deref for ErShared<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.value
    }
}
impl<T: ?Sized> AsRef<T> for ErShared<T> {
    fn as_ref(&self) -> &T {
        self
    }
}
impl<T: fmt::Debug + ?Sized> fmt::Debug for ErShared<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&**self, formatter)
    }
}
impl<T: fmt::Display + ?Sized> fmt::Display for ErShared<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&**self, formatter)
    }
}

macro_rules! borrowed_presentation {
    ($pointer:ident) => {
        impl<'a, P: ?Sized> IntoErSnapshot for &'a $pointer<P>
        where
            &'a P: IntoErSnapshot,
        {
            type Top = <&'a P as IntoErSnapshot>::Top;
            type Report = <&'a P as IntoErSnapshot>::Report;

            fn into_er_snapshot(self) -> ErSnapshot {
                <&'a P as IntoErSnapshot>::into_er_snapshot(self)
            }

            fn into_er_top(self) -> Self::Top {
                <&'a P as IntoErSnapshot>::into_er_top(self)
            }

            fn into_er_report(self) -> Self::Report {
                <&'a P as IntoErSnapshot>::into_er_report(self)
            }
        }
    };
}

borrowed_presentation!(Rc);
#[cfg(target_has_atomic = "ptr")]
borrowed_presentation!(Arc);
borrowed_presentation!(ErShared);

crate::impls::construct::saved_outputs!(ErSavedShared, Shared<A>, Shared::new);

#[cfg(all(er_unsync, target_has_atomic = "ptr"))]
crate::impls::construct::saved_outputs!(ErSavedArc, Arc<A>, Arc::new);
