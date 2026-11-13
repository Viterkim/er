use crate::aggregate::ErAggregateItems;
use crate::impls::node::find_below;
use crate::impls::saved::snapshot_saved;
use crate::{
    BoxError, ErAllItem, ErBoxedInput, ErBoxedTreeExt, ErEntries, ErErrorContextExt, ErErrorIndex,
    ErErrorWithSaved, ErFindAll, ErInput, ErNode, ErNodes, ErPart, ErReportError, ErSaveInput,
    ErSavedList, ErSnapshot, ErSources, ErTree, ErTreeContext, ErWithFields, InternalReport,
    IntoErPart, IntoErTree, IntoErTreeRef,
};
#[cfg(er_unsync)]
use alloc::boxed::Box;
use alloc::{string::String, vec::Vec};
#[cfg(feature = "src_locations")]
use core::panic::Location;
use core::{error::Error, fmt, ops::Deref};

impl<E: Error + 'static> ErTree<E> {
    /// Put one error or a list below this one. The list can be empty.
    #[track_caller]
    pub fn new<Mode, Kind>(
        error: E,
        items: impl ErAggregateItems<Kind, Item: ErAllItem<Mode>>,
    ) -> Self {
        Self::from(error).er_add(items)
    }

    /// Erase the top, keeping its sub errors and saved reports.
    pub fn into_er_part(self) -> ErPart
    where
        E: Into<BoxError>,
    {
        self.into_part(Into::into)
    }

    /// Returns the FIRST match.
    ///
    /// `error.er_find::<io::Error>()`
    pub fn er_find<T: Error + 'static>(&self) -> Option<&T> {
        self.internal_report().find()
    }

    /// True if this type is anywhere in the tree or its native sources.
    pub fn er_contains<T: Error + 'static>(&self) -> bool {
        self.er_find::<T>().is_some()
    }

    /// Finds all the instances of an error type, for when you have duplicates.
    ///
    /// `error.er_find_all::<PortErr>().find(|err| err.input == "aint_even_a_number_cmon_man")`
    pub fn er_find_all<T: Error + 'static>(&self) -> ErFindAll<'_, T> {
        ErFindAll::new(&self.top, &self.nodes)
    }
}
impl<E: ErReportError<ErBoxedInput>> ErBoxedTreeExt for ErTree<E> {
    fn er_find<T: Error + 'static>(&self) -> Option<&T> {
        self.internal_report().find()
    }

    fn er_find_all<T: Error + 'static>(&self) -> ErFindAll<'_, T> {
        ErFindAll::new(self.top.as_error(), &self.nodes)
    }
}

impl<'a> InternalReport<'a> {
    pub fn find<T: Error + 'static>(&self) -> Option<&'a T> {
        self.top
            .downcast_ref::<T>()
            .or_else(|| ErSources::new(self.top.source()).find_map(<dyn Error>::downcast_ref::<T>))
            .or_else(|| find_below(self.nodes))
    }
}
impl<E, Mode, Input> ErErrorContextExt<Mode, Input> for ErTree<E> where Self: IntoErPart<Input> {}
impl<E: Error + Into<BoxError> + 'static, Fields> ErTreeContext<(Fields, ())> for ErTree<E> {
    type Fields = Fields;

    #[track_caller]
    fn with_top<A: Error + 'static>(self, error: ErErrorWithSaved<A>) -> ErTree<A> {
        with_source(error, self)
    }
}
#[diagnostic::do_not_recommend]
impl<E, Fields> ErTreeContext<(Fields, ErBoxedInput)> for ErTree<E>
where
    Self: IntoErPart<ErBoxedInput>,
{
    type Fields = Fields;

    #[track_caller]
    fn with_top<A: Error + 'static>(self, error: ErErrorWithSaved<A>) -> ErTree<A> {
        with_source(error, self)
    }
}
impl<E> ErTree<E> {
    /// Build the new top from fields taken from the old top.
    ///
    /// `let error = error.er_with(|err| err.code);`
    #[track_caller]
    pub fn er_with<A, Mode>(
        self,
        fields: impl ErWithFields<E, A, <Self as ErTreeContext<Mode>>::Fields>,
    ) -> ErTree<A>
    where
        Self: ErTreeContext<Mode>,
        A: Error + 'static,
    {
        let error = fields.er_with_fields(&self.top);
        self.with_top(error)
    }

    /// Like `.er_with()`, but the closure gets the whole tree.
    #[track_caller]
    pub fn er_with_tree<A, Mode>(
        self,
        fields: impl ErWithFields<Self, A, <Self as ErTreeContext<Mode>>::Fields>,
    ) -> ErTree<A>
    where
        Self: ErTreeContext<Mode>,
        A: Error + 'static,
    {
        let error = fields.er_with_fields(&self);
        self.with_top(error)
    }

    fn into_part(self, box_top: impl FnOnce(E) -> BoxError) -> ErPart {
        ErPart {
            node: ErNode {
                error: box_top(self.top),
                nodes: self.nodes,
                #[cfg(feature = "src_locations")]
                src_location: self.src_location,
            },
            saved: self.saved,
        }
    }

    #[track_caller]
    pub(crate) fn from_top(top: E) -> Self {
        Self {
            top,
            nodes: Vec::new(),
            saved: ErSavedList::default(),
            #[cfg(feature = "src_locations")]
            src_location: Location::caller(),
        }
    }

    /// Look up a stored error by index. Native sources aren't counted.
    pub fn er_at_index<Input>(&self, index: ErErrorIndex) -> Option<&(dyn Error + 'static)>
    where
        E: ErReportError<Input>,
    {
        if index.0 == 0 {
            return Some(self.top.as_error());
        }

        let node = self.er_descendants().nth(index.0 - 1)?;
        Some(node.er_view().0)
    }

    /// Follow sub error indices. An empty path selects the root.
    pub fn er_at_path<Input>(&self, path: &[usize]) -> Option<&(dyn Error + 'static)>
    where
        E: ErReportError<Input>,
    {
        let Some((first, rest)) = path.split_first() else {
            return Some(self.top.as_error());
        };
        let mut node = self.nodes.get(*first)?;
        for index in rest {
            node = node.er_view().1.get(*index)?;
        }

        Some(node.er_view().0)
    }

    /// The report a line at a time, without line endings. Only formats once.
    pub fn for_each_line<Input>(&self, emit: impl FnMut(&str)) -> fmt::Result
    where
        E: ErReportError<Input>,
    {
        crate::lines::for_each_line(&self.internal_report(), emit)
    }

    /// Like `for_each_line()`, but the callback can fail.
    pub fn try_for_each_line<X, Input>(
        &self,
        emit: impl FnMut(&str) -> Result<(), X>,
    ) -> Result<(), crate::ErLineError<X>>
    where
        E: ErReportError<Input>,
    {
        crate::lines::try_for_each_line(&self.internal_report(), emit)
    }

    /// Every error, including native sources.
    pub fn er_entries<Input>(&self) -> ErEntries<'_>
    where
        E: ErReportError<Input>,
    {
        let report = self.internal_report();
        ErEntries::new(report.top, report.nodes, report.src_location)
    }

    /// Follows the root's `source()` chain.
    #[inline]
    pub fn er_sources<Input>(&self) -> ErSources<'_>
    where
        E: ErReportError<Input>,
    {
        ErSources::new(self.top.as_error().source())
    }

    /// Save the messages, tree structure and locations, without keeping the errors.
    /// A broken formatter's partial message is replaced with `ER_FMT_FAILED`.
    pub fn er_snapshot<Input>(&self) -> ErSnapshot
    where
        E: ErReportError<Input>,
    {
        let mut snapshot: ErSnapshot = self.er_entries().into();
        snapshot.saved = snapshot_saved(&self.saved);
        snapshot
    }

    pub(crate) fn internal_report<Input>(&self) -> InternalReport<'_>
    where
        E: ErReportError<Input>,
    {
        InternalReport::new(self)
    }

    /// The whole report as text, borrows the tree.
    #[must_use]
    pub fn er_report<Input>(&self) -> String
    where
        E: ErReportError<Input>,
    {
        self.internal_report().render_string()
    }

    /// Write the report straight to a buffer. The writer can return `fmt::Error` to stop early.
    pub fn er_write_report<Input>(&self, writer: &mut (impl fmt::Write + ?Sized)) -> fmt::Result
    where
        E: ErReportError<Input>,
    {
        writer.write_fmt(format_args!("{}", self.internal_report()))
    }

    /// Save another report on this tree.
    pub fn er_save(mut self, title: impl Into<String>, saved: impl ErSaveInput) -> Self {
        saved.er_save_into(title.into(), &mut self.saved);
        self
    }

    /// Add an error, tree or Result below the current top. Successful Results are skipped.
    /// Pass a collection or iterator to add several. Use `er_add!(tree, [first, second])` for different types.
    #[track_caller]
    #[inline]
    pub fn er_add<Mode, Kind>(
        self,
        items: impl ErAggregateItems<Kind, Item: ErAllItem<Mode>>,
    ) -> Self {
        // Bind in reverse field order so a panicking iterator still drops the top first.
        let Self {
            #[cfg(feature = "src_locations")]
            src_location,
            mut saved,
            mut nodes,
            top,
        } = self;

        crate::aggregate::append(&mut nodes, &mut saved, items);

        Self {
            top,
            nodes,
            saved,
            #[cfg(feature = "src_locations")]
            src_location,
        }
    }

    /// The stored sub errors, no root or native sources.
    pub fn er_descendants(&self) -> ErNodes<'_> {
        ErNodes::new(&self.nodes)
    }

    /// Take the top error and its whole report out of the tree.
    #[must_use]
    #[track_caller]
    pub fn er_split<Input>(self) -> (E, String)
    where
        E: ErReportError<Input>,
    {
        self.er_split_with(core::convert::identity)
    }

    /// Give the top error and its report to the closure.
    #[track_caller]
    pub fn er_split_with<A, Input>(self, into: impl FnOnce((E, String)) -> A) -> A
    where
        E: ErReportError<Input>,
    {
        let report = self.er_report();
        let Self {
            top, nodes, saved, ..
        } = self;

        // The output stays a local while the rest drops, a panicking destructor down there would leak it
        // out of the return slot.
        let output = into((top, report));
        drop(nodes);
        drop(saved);

        output
    }
}
impl<E> IntoErTree for ErTree<E> {
    type Error = E;

    fn into_er_tree(self) -> Self {
        self
    }
}
impl<'a, E> IntoErTreeRef<'a> for &'a ErTree<E> {
    type Error = E;

    fn into_er_tree_ref(self) -> &'a ErTree<Self::Error> {
        self
    }
}
impl<'a, 'b, P: Deref + ?Sized> IntoErTreeRef<'a> for &'b P
where
    &'b P::Target: IntoErTreeRef<'a>,
{
    type Error = <&'b P::Target as IntoErTreeRef<'a>>::Error;

    fn into_er_tree_ref(self) -> &'a ErTree<Self::Error> {
        <&'b P::Target as IntoErTreeRef<'a>>::into_er_tree_ref(self)
    }
}
impl<'a, 'b, P: ?Sized> IntoErTreeRef<'a> for &'b mut P
where
    &'b P: IntoErTreeRef<'a>,
{
    type Error = <&'b P as IntoErTreeRef<'a>>::Error;

    fn into_er_tree_ref(self) -> &'a ErTree<Self::Error> {
        <&'b P as IntoErTreeRef<'a>>::into_er_tree_ref(self)
    }
}
impl<E: Error + 'static> From<E> for ErTree<E> {
    #[track_caller]
    fn from(error: E) -> Self {
        Self::from_top(error)
    }
}
impl<E: Error + Into<BoxError> + 'static> IntoErPart for ErTree<E> {
    type Error = E;

    fn er_error(&self) -> &E {
        &self.top
    }

    #[inline]
    fn into_er_part(self) -> ErPart {
        ErTree::into_er_part(self)
    }
}

macro_rules! boxed_top {
    ($top:ty) => {
        impl IntoErPart<ErBoxedInput> for ErTree<$top> {
            type Error = $top;

            fn er_error(&self) -> &Self::Error {
                &self.top
            }

            #[inline]
            fn into_er_part(self) -> ErPart {
                self.into_part(|top| top)
            }
        }
    };
}
boxed_top!(BoxError);
#[cfg(er_unsync)]
boxed_top!(Box<dyn Error + Send + Sync>);
#[cfg(er_unsync)]
boxed_top!(Box<dyn Error + Send>);
#[cfg(er_unsync)]
boxed_top!(Box<dyn Error + Sync>);

// Cold, so the Vec work stays out of the caller's success path. Building empty contexts inline saves a few ns
// per failure, but every function with an `.er(())` grows. With an error type per function the successful
// calls then run 40% slower, and up to 5 times slower when small functions get too big to inline.
#[cold]
#[track_caller]
pub fn with_source<E, Input>(error: ErErrorWithSaved<E>, source: impl ErInput<Input>) -> ErTree<E>
where
    E: Error + 'static,
{
    // Allocate first so the node can go straight into its slot.
    let mut nodes = Vec::with_capacity(1);
    let ErPart { node, mut saved } = source.into_er_input();
    nodes.push(node);

    let ErErrorWithSaved {
        value: top,
        saved: mut extra,
    } = error;
    saved.append(&mut extra);

    ErTree {
        top,
        nodes,
        saved,
        #[cfg(feature = "src_locations")]
        src_location: Location::caller(),
    }
}
