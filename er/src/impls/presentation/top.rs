use crate::lines;
use crate::render::{format_string, write_top};
use crate::{
    BoxError, ErEntries, ErEntry, ErErrorIndex, ErFindAll, ErLineError, ErMake, ErNodes, ErReport,
    ErReportRef, ErSnapshot, ErSources, ErTop, ErTopRef, ErTree, ErTreeContextExt, IntoErPart,
    IntoErTree, IntoErTreeRef, Layout,
};
use alloc::string::String;
use core::{error::Error, fmt};

impl<'a, E> ErTopRef<'a, E> {
    /// The same top, keeping its layout.
    pub const fn er_top(&self) -> Self {
        *self
    }

    /// The whole report, keeping this layout.
    pub const fn er_report(&self) -> ErReportRef<'a, E> {
        ErReportRef {
            tree: self.tree,
            layout: self.layout,
        }
    }

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
impl<'a, E> From<&'a ErTree<E>> for ErTopRef<'a, E> {
    fn from(tree: &'a ErTree<E>) -> Self {
        tree.er_top()
    }
}
impl<'a, E> From<ErReportRef<'a, E>> for ErTopRef<'a, E> {
    fn from(report: ErReportRef<'a, E>) -> Self {
        report.er_top()
    }
}
impl<E> Copy for ErTopRef<'_, E> {}
impl<E> Clone for ErTopRef<'_, E> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<'a, E> IntoErTreeRef<'a> for ErTopRef<'a, E> {
    type Error = E;

    fn into_er_report_ref(self) -> ErReportRef<'a, E> {
        self.er_report()
    }
}
impl<'a, E> IntoErTreeRef<'a> for &ErTopRef<'a, E> {
    type Error = E;

    fn into_er_report_ref(self) -> ErReportRef<'a, E> {
        self.er_report()
    }
}
impl<E: fmt::Display> ErTopRef<'_, E> {
    /// Just the outer error as text, keeps this layout.
    pub fn er_top_string(&self) -> String {
        format_string(self)
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
}
impl<'a, E: Error + 'static> ErTopRef<'a, E> {
    /// Look up a stored error by index. Native sources aren't counted.
    pub fn er_at_index(&self, index: ErErrorIndex) -> Option<&'a (dyn Error + 'static)> {
        self.tree.er_at_index(index)
    }

    /// Follow child indices. An empty path selects the root.
    pub fn er_at_path(&self, path: &[usize]) -> Option<&'a (dyn Error + 'static)> {
        self.tree.er_at_path(path)
    }

    /// Every error, including native sources.
    pub fn er_entries(&self) -> ErEntries<'a> {
        self.tree.er_entries()
    }

    /// Call once per error, including native sources.
    pub fn er_for_each_entry(&self, visit: impl FnMut(ErEntry<'a>)) {
        self.tree.er_for_each_entry(visit);
    }

    /// The whole report as text, keeps this layout.
    pub fn er_report_string(&self) -> String {
        ErReportRef {
            tree: self.tree,
            layout: self.layout,
        }
        .er_report_string()
    }

    /// Save the whole tree, even when printing just the top.
    pub fn er_snapshot(&self) -> ErSnapshot {
        self.tree.er_snapshot()
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
    pub fn er_find_all<T: Error + 'static>(&self) -> ErFindAll<'a, T> {
        self.tree.er_find_all::<T>()
    }

    /// True if this type is anywhere in the tree or its native sources.
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

impl<E> ErTop<E> {
    /// Borrow the same top, keeping its layout.
    pub const fn er_top(&self) -> ErTopRef<'_, E> {
        self.as_ref()
    }

    /// The whole report, keeping this layout.
    pub const fn er_report(&self) -> ErReportRef<'_, E> {
        ErReportRef {
            tree: &self.tree,
            layout: self.layout,
        }
    }

    /// Pick how it gets printed.
    pub fn layout(self, layout: Layout) -> Self {
        Self { layout, ..self }
    }

    /// Print on one line.
    pub fn single_line(self) -> Self {
        self.layout(Layout::SingleLine)
    }

    /// Borrow the same view, keeping its layout.
    pub const fn as_ref(&self) -> ErTopRef<'_, E> {
        ErTopRef {
            tree: &self.tree,
            layout: self.layout,
        }
    }

    /// Stored sub errors, skips the root and native sources.
    pub fn er_descendants(&self) -> ErNodes<'_> {
        self.tree.er_descendants()
    }
}
impl<E: Error + 'static> From<E> for ErTop<E> {
    #[track_caller]
    fn from(error: E) -> Self {
        ErTree::from(error).into_er_top()
    }
}
impl<E> From<ErTree<E>> for ErTop<E> {
    fn from(tree: ErTree<E>) -> Self {
        tree.into_er_top()
    }
}
impl<E> From<ErReport<E>> for ErTop<E> {
    fn from(report: ErReport<E>) -> Self {
        report.into_er_top()
    }
}
impl<E> IntoErTree for ErTop<E> {
    type Error = E;

    fn into_er_tree(self) -> ErTree<E> {
        self.tree
    }

    fn into_er_report(self) -> ErReport<E> {
        ErReport {
            tree: self.tree,
            layout: self.layout,
        }
    }
}
impl<E> AsMut<ErTree<E>> for ErTop<E> {
    fn as_mut(&mut self) -> &mut ErTree<E> {
        &mut self.tree
    }
}
impl<'a, E> IntoErTreeRef<'a> for &'a ErTop<E> {
    type Error = E;

    fn into_er_report_ref(self) -> ErReportRef<'a, E> {
        self.er_report()
    }
}
impl<E: Error + Into<BoxError> + 'static, Mode> ErTreeContextExt<Mode> for ErTop<E> {
    #[track_caller]
    fn er<A>(self, top: impl ErMake<A, Mode>) -> ErTree<A>
    where
        A: Error + 'static,
    {
        self.tree.er(top)
    }
}
impl<E: fmt::Display> ErTop<E> {
    /// Just the outer error as text, keeps this layout.
    pub fn er_top_string(&self) -> String {
        self.as_ref().er_top_string()
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
}
impl<E: Error + 'static> ErTop<E> {
    /// Look up a stored error by index. Native sources aren't counted.
    pub fn er_at_index(&self, index: ErErrorIndex) -> Option<&(dyn Error + 'static)> {
        self.tree.er_at_index(index)
    }

    /// Follow child indices. An empty path selects the root.
    pub fn er_at_path(&self, path: &[usize]) -> Option<&(dyn Error + 'static)> {
        self.tree.er_at_path(path)
    }

    /// Every error, including native sources.
    pub fn er_entries(&self) -> ErEntries<'_> {
        self.tree.er_entries()
    }

    /// Call once per error, including native sources.
    pub fn er_for_each_entry<'a>(&'a self, visit: impl FnMut(ErEntry<'a>)) {
        self.tree.er_for_each_entry(visit);
    }

    /// The whole report as text, keeps this layout.
    pub fn er_report_string(&self) -> String {
        self.as_ref().er_report_string()
    }

    /// Save the whole tree, even when printing just the top.
    pub fn er_snapshot(&self) -> ErSnapshot {
        self.tree.er_snapshot()
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
    pub fn er_find_all<T: Error + 'static>(&self) -> ErFindAll<'_, T> {
        self.tree.er_find_all::<T>()
    }

    /// True if this type is anywhere in the tree or its native sources.
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
impl<E: Error + Into<BoxError> + 'static> IntoErPart for ErTop<E> {
    type Error = E;

    fn er_error(&self) -> &E {
        &self.tree.top
    }

    fn into_er_part(self) -> crate::ErPart {
        self.tree.into_er_part()
    }
}
