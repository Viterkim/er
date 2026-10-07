#[cfg(feature = "stack_traces")]
use crate::impls::stack_trace::append_traces;
use crate::{
    BoxError, ErEntries, ErEntry, ErErrorIndex, ErFindAll, ErInput, ErMake, ErNode, ErNodes,
    ErPart, ErReport, ErReportRef, ErSnapshot, ErSources, ErTop, ErTopRef, ErTree,
    ErTreeContextExt, ErWithFields, IntoErPart, IntoErTree, Layout,
};
use alloc::{string::String, vec, vec::Vec};
#[cfg(feature = "src_locations")]
use core::panic::Location;
use core::{error::Error, fmt};

impl<E: Error + 'static> ErTree<E> {
    /// Put existing errors below this one, even if the list is empty.
    #[cfg_attr(feature = "src_locations", track_caller)]
    pub fn new<Input>(error: E, nodes: impl IntoIterator<Item = impl ErInput<Input>>) -> Self {
        Self::from(error).er_add(nodes)
    }

    /// Build the new top from fields taken from the old top.
    ///
    /// `let error = error.er_with(|err| err.code);`
    #[cfg_attr(feature = "src_locations", track_caller)]
    pub fn er_with<A, Mode>(self, fields: impl ErWithFields<E, A, Mode>) -> ErTree<A>
    where
        E: Into<BoxError>,
        A: Error + 'static,
    {
        let error = fields.er_with_fields(&self.top);
        with_source(error, self)
    }

    /// Like `.er_with()`, but the closure gets the whole tree.
    #[cfg_attr(feature = "src_locations", track_caller)]
    pub fn er_with_tree<A, Mode>(self, fields: impl ErWithFields<Self, A, Mode>) -> ErTree<A>
    where
        E: Into<BoxError>,
        A: Error + 'static,
    {
        let error = fields.er_with_fields(&self);
        with_source(error, self)
    }

    /// Erase the top error. This drops its stack traces, use `into_er_part()` to keep them.
    pub fn into_er_node(self) -> ErNode
    where
        E: Into<BoxError>,
    {
        self.into_er_part().node
    }

    /// Erase the top, keeping its children and stack traces.
    pub fn into_er_part(self) -> ErPart
    where
        E: Into<BoxError>,
    {
        let error = self.top.into();

        ErPart {
            node: ErNode {
                error,
                nodes: self.nodes,
                #[cfg(feature = "src_locations")]
                src_location: self.src_location,
            },
            #[cfg(feature = "stack_traces")]
            stack_traces: self.stack_traces,
        }
    }

    /// Every error, including native sources.
    pub fn er_entries(&self) -> ErEntries<'_> {
        ErEntries::new(
            &self.top,
            &self.nodes,
            #[cfg(feature = "src_locations")]
            Some(self.src_location),
            #[cfg(not(feature = "src_locations"))]
            None,
        )
    }

    /// Call once per error, including native sources.
    pub fn er_for_each_entry<'a>(&'a self, visit: impl FnMut(ErEntry<'a>)) {
        self.er_entries().for_each(visit);
    }

    /// Follows the root's `source()` chain.
    #[inline]
    pub fn er_sources(&self) -> ErSources<'_> {
        ErSources::new(self.top.source())
    }

    /// Returns the FIRST match.
    ///
    /// `error.er_find::<io::Error>()`
    pub fn er_find<T: Error + 'static>(&self) -> Option<&T> {
        let error: &(dyn Error + 'static) = &self.top;

        error
            .downcast_ref::<T>()
            .or_else(|| self.er_sources().find_map(<dyn Error>::downcast_ref::<T>))
            .or_else(|| self.nodes.iter().find_map(ErNode::er_find::<T>))
    }

    /// True if this type is anywhere in the tree or its native sources.
    pub fn er_contains<T: Error + 'static>(&self) -> bool {
        self.er_find::<T>().is_some()
    }

    /// Look up a stored error by index. Native sources aren't counted.
    pub fn er_at_index(&self, index: ErErrorIndex) -> Option<&(dyn Error + 'static)> {
        if index.0 == 0 {
            return Some(&self.top);
        }

        let node = self.er_descendants().nth(index.0 - 1)?;
        Some(&*node.error)
    }

    /// Follow child indices. An empty path selects the root.
    pub fn er_at_path(&self, path: &[usize]) -> Option<&(dyn Error + 'static)> {
        let Some((first, rest)) = path.split_first() else {
            return Some(&self.top);
        };
        let mut node = self.nodes.get(*first)?;
        for index in rest {
            node = node.nodes.get(*index)?;
        }

        Some(&*node.error)
    }

    /// Finds all the instances of an error type, for when you have duplicates.
    ///
    /// `error.er_find_all::<PortErr>().find(|e| e.input == "aint_even_a_number_cmon_man")`
    pub fn er_find_all<T: Error + 'static>(&self) -> ErFindAll<'_, T> {
        ErFindAll::new(&self.top, &self.nodes)
    }

    /// Save the messages, tree structure and locations, without keeping the errors.
    /// A broken formatter's partial message is replaced with `ER_FMT_FAILED`.
    pub fn er_snapshot(&self) -> ErSnapshot {
        self.er_entries().into()
    }

    /// The whole report as text, borrows the tree.
    pub fn er_report_string(&self) -> String {
        self.er_report().er_report_string()
    }
}
impl<E: Error + Into<BoxError> + 'static, Mode> ErTreeContextExt<Mode> for ErTree<E> {
    #[cfg_attr(feature = "src_locations", track_caller)]
    fn er<A>(self, top: impl ErMake<A, Mode>) -> ErTree<A>
    where
        A: Error + 'static,
    {
        with_source(top.er_make(), self)
    }
}
impl<E> ErTree<E> {
    /// Add errors or subtrees below the current top.
    /// Use `er_add!(tree, [first, second])` for different types.
    #[cfg_attr(feature = "src_locations", track_caller)]
    pub fn er_add<Input>(mut self, nodes: impl IntoIterator<Item = impl ErInput<Input>>) -> Self {
        let nodes = nodes.into_iter();
        if self.nodes.capacity() == 0 {
            self.nodes.reserve_exact(nodes.size_hint().0);
        } else {
            self.nodes.reserve(nodes.size_hint().0);
        }
        #[cfg(feature = "stack_traces")]
        let (mut counted, mut next_index) = (0, 1);

        for node in nodes {
            let part = ErInput::into_er_input(node);
            #[cfg(feature = "stack_traces")]
            {
                let mut traces = part.stack_traces;
                if !traces.is_empty() {
                    for node in &self.nodes[counted..] {
                        next_index += 1 + node.er_descendants().count();
                    }
                    counted = self.nodes.len();
                    for trace in &mut traces {
                        trace.error_index.0 += next_index;
                    }
                }
                append_traces(&mut self.stack_traces, traces);
            }
            self.nodes.push(part.node);
        }

        self
    }

    /// The stored sub errors, no root or native sources.
    pub fn er_descendants(&self) -> ErNodes<'_> {
        ErNodes::new(&self.nodes)
    }

    /// Just the outer error, borrows the tree.
    ///
    /// `println!("{}", error.er_top());`
    pub const fn er_top(&self) -> ErTopRef<'_, E> {
        ErTopRef {
            tree: self,
            layout: Layout::Multiline,
        }
    }

    /// The whole report, borrows the tree.
    ///
    /// `println!("{}", error.er_report());`
    pub const fn er_report(&self) -> ErReportRef<'_, E> {
        ErReportRef {
            tree: self,
            layout: Layout::Multiline,
        }
    }

    /// Just the outer error, moves the tree.
    pub const fn into_er_top(self) -> ErTop<E> {
        ErTop {
            tree: self,
            layout: Layout::Multiline,
        }
    }

    /// The whole report, moves the tree.
    pub const fn into_er_report(self) -> ErReport<E> {
        ErReport {
            tree: self,
            layout: Layout::Multiline,
        }
    }
}
impl<E: fmt::Display> ErTree<E> {
    /// Just the outer error as text, borrows the tree.
    pub fn er_top_string(&self) -> String {
        self.er_top().er_top_string()
    }
}
impl<E> IntoErTree for ErTree<E> {
    type Error = E;

    fn into_er_tree(self) -> Self {
        self
    }
}
impl<E> AsMut<ErTree<E>> for ErTree<E> {
    fn as_mut(&mut self) -> &mut Self {
        self
    }
}
impl<E> From<ErTop<E>> for ErTree<E> {
    fn from(top: ErTop<E>) -> Self {
        top.tree
    }
}
impl<E> From<ErReport<E>> for ErTree<E> {
    fn from(report: ErReport<E>) -> Self {
        report.tree
    }
}
impl<E: Error + 'static> From<E> for ErTree<E> {
    #[cfg_attr(feature = "src_locations", track_caller)]
    fn from(error: E) -> Self {
        Self {
            top: error,
            nodes: Vec::new(),
            #[cfg(feature = "stack_traces")]
            stack_traces: Vec::new(),
            #[cfg(feature = "src_locations")]
            src_location: Location::caller(),
        }
    }
}
impl<E: Error + Into<BoxError> + 'static> IntoErPart for ErTree<E> {
    type Error = E;

    fn er_error(&self) -> &E {
        &self.top
    }

    fn into_er_part(self) -> ErPart {
        ErTree::into_er_part(self)
    }
}

// Keep the Vec work out of the caller's success path.
#[cold]
#[inline(never)]
#[cfg_attr(feature = "src_locations", track_caller)]
pub fn with_source<E, Input>(error: E, source: impl ErInput<Input>) -> ErTree<E>
where
    E: Error + 'static,
{
    let mut tree = ErTree::from(error);
    let part = source.into_er_input();

    #[cfg(feature = "stack_traces")]
    {
        let mut traces = part.stack_traces;
        for trace in &mut traces {
            trace.error_index.0 += 1;
        }
        tree.stack_traces = traces;
    }

    tree.nodes = vec![part.node];

    tree
}
