use crate::lines;
use crate::render::{format_string, report::write_entries};
use crate::{
    BoxError, ErAsError, ErEntries, ErEntry, ErLineError, ErMake, ErNodes, ErOpaqueErrorExt,
    ErReport, ErReportRef, ErSnapshot, ErSources, ErTopRef, ErTree, ErTreeContextExt, IntoErPart,
    IntoErTree, Layout,
};
use alloc::string::String;
use core::{error::Error, fmt};

impl<'a, E> ErReportRef<'a, E> {
    /// Pick how it gets printed.
    pub const fn layout(self, layout: Layout) -> Self {
        Self { layout, ..self }
    }

    /// Print on one line.
    pub const fn single_line(self) -> Self {
        self.layout(Layout::SingleLine)
    }

    /// Stored sub errors, skips the root and native sources.
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
    /// The report as text, keeps this layout.
    pub fn er_report_string(&self) -> String {
        format_string(self)
    }

    /// Just the outer error as text, keeps this layout.
    pub fn er_top_string(&self) -> String {
        ErTopRef {
            tree: self.tree,
            layout: self.layout,
        }
        .er_top_string()
    }

    /// Save the whole tree as messages, keeps its structure.
    pub fn er_snapshot(&self) -> ErSnapshot {
        self.tree.er_snapshot()
    }

    /// Only formats once, no line endings.
    pub fn for_each_line(&self, emit: impl FnMut(&str)) -> fmt::Result {
        lines::for_each_line(self, emit)
    }

    /// Like `for_each_line()`, but the callback can fail.
    pub fn try_for_each_line<X>(
        &self,
        emit: impl FnMut(&str) -> Result<(), X>,
    ) -> Result<(), ErLineError<X>> {
        lines::try_for_each_line(self, emit)
    }

    /// Every error, including native sources.
    pub fn er_entries(&self) -> ErEntries<'a> {
        self.tree.er_entries()
    }

    /// Calls once per error. An error's message can contain several lines.
    pub fn er_for_each_entry(&self, visit: impl FnMut(ErEntry<'a>)) {
        self.tree.er_for_each_entry(visit);
    }

    /// The top error's native `source()` chain.
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

    /// True if this type is anywhere in the tree or its native sources.
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
impl<E: Error + 'static> ErOpaqueErrorExt for ErReportRef<'_, E> {
    type Output = ErAsError<Self>;

    fn opaque_err(self) -> Self::Output {
        ErAsError(self)
    }
}

impl<E> ErReport<E> {
    /// Pick how it gets printed.
    pub fn layout(self, layout: Layout) -> Self {
        Self { layout, ..self }
    }

    /// Print on one line.
    pub fn single_line(self) -> Self {
        self.layout(Layout::SingleLine)
    }

    /// Borrow the same view, keeping its layout.
    pub const fn as_ref(&self) -> ErReportRef<'_, E> {
        ErReportRef {
            tree: &self.tree,
            layout: self.layout,
        }
    }

    /// Stored sub errors, skips the root and native sources.
    pub fn er_descendants(&self) -> ErNodes<'_> {
        self.tree.er_descendants()
    }
}
impl<E: Error + 'static> From<E> for ErReport<E> {
    #[cfg_attr(feature = "src_locations", track_caller)]
    fn from(error: E) -> Self {
        ErTree::from(error).into_er_report()
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

    fn into_er_report(self) -> Self {
        self
    }
}
impl<E> AsMut<ErTree<E>> for ErReport<E> {
    fn as_mut(&mut self) -> &mut ErTree<E> {
        &mut self.tree
    }
}
impl<E: Error + Into<BoxError> + 'static, Mode> ErTreeContextExt<Mode> for ErReport<E> {
    #[cfg_attr(feature = "src_locations", track_caller)]
    fn er<A>(self, top: impl ErMake<A, Mode>) -> ErTree<A>
    where
        A: Error + 'static,
    {
        self.tree.er(top)
    }
}
impl<E: Error + 'static> ErReport<E> {
    /// The report as text, keeps this layout.
    pub fn er_report_string(&self) -> String {
        self.as_ref().er_report_string()
    }

    /// Just the outer error as text, keeps this layout.
    pub fn er_top_string(&self) -> String {
        self.as_ref().er_top_string()
    }

    /// Save the whole tree as messages, keeps its structure.
    pub fn er_snapshot(&self) -> ErSnapshot {
        self.tree.er_snapshot()
    }

    /// Only formats once, no line endings.
    pub fn for_each_line(&self, emit: impl FnMut(&str)) -> fmt::Result {
        lines::for_each_line(self, emit)
    }

    /// Like `for_each_line()`, but the callback can fail.
    pub fn try_for_each_line<X>(
        &self,
        emit: impl FnMut(&str) -> Result<(), X>,
    ) -> Result<(), ErLineError<X>> {
        lines::try_for_each_line(self, emit)
    }

    /// Every error, including native sources.
    pub fn er_entries(&self) -> ErEntries<'_> {
        self.tree.er_entries()
    }

    /// Calls once per error. An error's message can contain several lines.
    pub fn er_for_each_entry<'a>(&'a self, visit: impl FnMut(ErEntry<'a>)) {
        self.tree.er_for_each_entry(visit);
    }

    /// The top error's native `source()` chain.
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

    /// True if this type is anywhere in the tree or its native sources.
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
impl<E: Error + Into<BoxError> + 'static> IntoErPart for ErReport<E> {
    type Error = E;

    fn er_error(&self) -> &E {
        &self.tree.top
    }

    fn into_er_part(self) -> crate::ErPart {
        self.tree.into_er_part()
    }
}
impl<E: Error + 'static> ErOpaqueErrorExt for ErReport<E> {
    type Output = ErAsError<Self>;

    fn opaque_err(self) -> Self::Output {
        ErAsError(self)
    }
}
