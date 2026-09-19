use crate::lines;
use crate::render::report::write_entries;
use crate::{
    ErAsError, ErEntries, ErEntry, ErLineError, ErNodes, ErOpaqueError, ErReport, ErReportRef,
    ErSources, ErTree, IntoErNode, IntoErTree, Layout,
};
use core::{error::Error, fmt};

impl<'a, E> ErReportRef<'a, E> {
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
impl<'a, E> From<&'a ErTree<E>> for ErReportRef<'a, E> {
    fn from(tree: &'a ErTree<E>) -> Self {
        tree.er_report()
    }
}
impl<E> Copy for ErReportRef<'_, E> {}
impl<E> Clone for ErReportRef<'_, E> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<'a, E: Error + 'static> ErReportRef<'a, E> {
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

    pub fn er_entries(&self) -> ErEntries<'a> {
        self.tree.er_entries()
    }

    /// Calls once per error. An error's message can contain several lines.
    pub fn er_for_each_entry(&self, visit: impl FnMut(ErEntry<'a>)) {
        self.tree.er_for_each_entry(visit);
    }

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
impl<E: Error + 'static> fmt::Display for ErReportRef<'_, E> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_entries(formatter, self.tree.er_entries(), self.layout)
    }
}
impl<E: Error + 'static> fmt::Debug for ErReportRef<'_, E> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, formatter)
    }
}
impl<E> ErReport<E> {
    pub fn layout(self, layout: Layout) -> Self {
        Self { layout, ..self }
    }

    pub fn single_line(self) -> Self {
        self.layout(Layout::SingleLine)
    }

    pub const fn as_ref(&self) -> ErReportRef<'_, E> {
        ErReportRef {
            tree: &self.tree,
            layout: self.layout,
        }
    }

    pub fn er_descendants(&self) -> ErNodes<'_> {
        self.tree.er_descendants()
    }
}
impl<E> From<ErTree<E>> for ErReport<E> {
    fn from(tree: ErTree<E>) -> Self {
        tree.into_er_report()
    }
}
impl<E> IntoErTree for ErReport<E> {
    type Error = E;

    fn into_er_tree(self) -> ErTree<E> {
        self.tree
    }
}
impl<E: Error + 'static> ErReport<E> {
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

    pub fn er_entries(&self) -> ErEntries<'_> {
        self.tree.er_entries()
    }

    /// Calls once per error. An error's message can contain several lines.
    pub fn er_for_each_entry<'a>(&'a self, visit: impl FnMut(ErEntry<'a>)) {
        self.tree.er_for_each_entry(visit);
    }

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
impl<E: Error + 'static> fmt::Display for ErReport<E> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.as_ref(), formatter)
    }
}
impl<E: Error + 'static> fmt::Debug for ErReport<E> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, formatter)
    }
}
impl<E: Error + Send + Sync + 'static> IntoErNode for ErReport<E> {
    fn into_er_node(self) -> crate::ErNode {
        self.tree.into_er_node()
    }
}

impl<E: Error + 'static> ErOpaqueError for ErReport<E> {
    type Output = ErAsError<Self>;

    fn opaque_err(self) -> Self::Output {
        ErAsError(self)
    }
}

impl<E: Error + 'static> ErOpaqueError for ErReportRef<'_, E> {
    type Output = ErAsError<Self>;

    fn opaque_err(self) -> Self::Output {
        ErAsError(self)
    }
}
