use crate::ErShared;
#[cfg(er_unsync)]
use alloc::rc::Rc as Shared;
#[cfg(not(er_unsync))]
use alloc::sync::Arc as Shared;
use core::{
    borrow::Borrow,
    cmp::Ordering,
    fmt,
    hash::{Hash, Hasher},
    ops::Deref,
};

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
// A `PathBuf` or `String` field takes the borrowed form too, and wrapping it in ErShared shouldn't take that away.
// An `AsRef<Path>` version would collide with `From<&T>` on a `&PathBuf`, `.er(path.as_ref())` covers it.
impl From<&str> for ErShared<alloc::string::String> {
    fn from(value: &str) -> Self {
        Self::new(value.into())
    }
}
#[cfg(feature = "std")]
impl From<&std::path::Path> for ErShared<std::path::PathBuf> {
    fn from(value: &std::path::Path) -> Self {
        Self::new(value.to_path_buf())
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
impl<T: ?Sized> Borrow<T> for ErShared<T> {
    fn borrow(&self) -> &T {
        self
    }
}
// Compare and hash the value like Arc does, so a type holding one can derive these.
impl<T: PartialEq + ?Sized> PartialEq for ErShared<T> {
    fn eq(&self, other: &Self) -> bool {
        **self == **other
    }
}
impl<T: Eq + ?Sized> Eq for ErShared<T> {}
impl<T: PartialOrd + ?Sized> PartialOrd for ErShared<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        (**self).partial_cmp(&**other)
    }
}
impl<T: Ord + ?Sized> Ord for ErShared<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        (**self).cmp(&**other)
    }
}
impl<T: Hash + ?Sized> Hash for ErShared<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (**self).hash(state);
    }
}
impl<T: Default> Default for ErShared<T> {
    fn default() -> Self {
        Self::new(T::default())
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
