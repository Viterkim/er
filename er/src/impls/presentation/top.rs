use crate::lines;
use crate::render::write_top;
use crate::{
    ErAsError, ErLineError, ErNodes, ErOpaqueError, ErSources, ErTop, ErTopRef, ErTree, IntoErPart,
    IntoErTree, Layout,
};
use core::{error::Error, fmt};

impl<'a, E> ErTopRef<'a, E> {
    pub const fn layout(self, layout: Layout) -> Self {
        Self { layout, ..self }
    }

    pub const fn single_line(self) -> Self {
        self.layout(Layout::SingleLine)
    }

    pub fn er_descendants(&self) -> ErNodes<'a> {
        self.tree.er_descendants()
    }
}
impl<'a, E> From<&'a ErTree<E>> for ErTopRef<'a, E> {
    fn from(tree: &'a ErTree<E>) -> Self {
        tree.er_top()
    }
}
impl<E> Copy for ErTopRef<'_, E> {}
impl<E> Clone for ErTopRef<'_, E> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<E: fmt::Display> ErTopRef<'_, E> {
    /// Only formats once, no line endings.
    pub fn for_each_line(&self, emit: impl FnMut(&str)) -> fmt::Result {
        lines::for_each_line(self, emit)
    }

    pub fn try_for_each_line<X>(
        &self,
        emit: impl FnMut(&str) -> Result<(), X>,
    ) -> Result<(), ErLineError<X>> {
        lines::try_for_each_line(self, emit)
    }
}
impl<'a, E: Error + 'static> ErTopRef<'a, E> {
    pub fn er_sources(&self) -> ErSources<'a> {
        self.tree.er_sources()
    }

    /// Returns the FIRST match.
    pub fn er_find<T: Error + 'static>(&self) -> Option<&'a T> {
        self.tree.er_find::<T>()
    }

    /// Finds all the instances of an error type, for when you have duplicates.
    pub fn er_find_all<T: Error + 'static>(&self) -> impl Iterator<Item = &'a T> + use<'a, E, T> {
        self.tree.er_find_all::<T>()
    }

    pub fn er_contains<T: Error + 'static>(&self) -> bool {
        self.tree.er_contains::<T>()
    }
}
impl<E: fmt::Display> fmt::Display for ErTopRef<'_, E> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_top(formatter, &self.tree.top, self.layout)
    }
}
impl<E: fmt::Display> fmt::Debug for ErTopRef<'_, E> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, formatter)
    }
}
impl<E: fmt::Display> ErOpaqueError for ErTopRef<'_, E> {
    type Output = ErAsError<Self>;

    fn opaque_err(self) -> Self::Output {
        ErAsError(self)
    }
}

impl<E> ErTop<E> {
    pub fn layout(self, layout: Layout) -> Self {
        Self { layout, ..self }
    }

    pub fn single_line(self) -> Self {
        self.layout(Layout::SingleLine)
    }

    pub const fn as_ref(&self) -> ErTopRef<'_, E> {
        ErTopRef {
            tree: &self.tree,
            layout: self.layout,
        }
    }

    pub fn er_descendants(&self) -> ErNodes<'_> {
        self.tree.er_descendants()
    }
}
impl<E> From<ErTree<E>> for ErTop<E> {
    fn from(tree: ErTree<E>) -> Self {
        tree.into_er_top()
    }
}
impl<E> IntoErTree for ErTop<E> {
    type Error = E;

    fn into_er_tree(self) -> ErTree<E> {
        self.tree
    }
}
impl<E: fmt::Display> ErTop<E> {
    /// Only formats once, no line endings.
    pub fn for_each_line(&self, emit: impl FnMut(&str)) -> fmt::Result {
        lines::for_each_line(self, emit)
    }

    pub fn try_for_each_line<X>(
        &self,
        emit: impl FnMut(&str) -> Result<(), X>,
    ) -> Result<(), ErLineError<X>> {
        lines::try_for_each_line(self, emit)
    }
}
impl<E: Error + 'static> ErTop<E> {
    pub fn er_sources(&self) -> ErSources<'_> {
        self.tree.er_sources()
    }

    /// Returns the FIRST match.
    pub fn er_find<T: Error + 'static>(&self) -> Option<&T> {
        self.tree.er_find::<T>()
    }

    /// Finds all the instances of an error type, for when you have duplicates.
    pub fn er_find_all<T: Error + 'static>(&self) -> impl Iterator<Item = &T> {
        self.tree.er_find_all::<T>()
    }

    pub fn er_contains<T: Error + 'static>(&self) -> bool {
        self.tree.er_contains::<T>()
    }
}
impl<E: fmt::Display> fmt::Display for ErTop<E> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.as_ref(), formatter)
    }
}
impl<E: fmt::Display> fmt::Debug for ErTop<E> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, formatter)
    }
}
impl<E: Error + Send + Sync + 'static> IntoErPart for ErTop<E> {
    fn into_er_part(self) -> crate::ErPart {
        self.tree.into_er_part()
    }
}
impl<E: fmt::Display> ErOpaqueError for ErTop<E> {
    type Output = ErAsError<Self>;

    fn opaque_err(self) -> Self::Output {
        ErAsError(self)
    }
}
