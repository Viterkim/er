#[cfg(feature = "src_locations")]
use crate::ErSnapshotLocation;
#[cfg(feature = "stack_traces")]
use crate::impls::stack_trace::append_traces;
use crate::{
    ErEntries, ErEntry, ErErrorIndex, ErFindAll, ErMake, ErNode, ErNodes, ErPart, ErReport,
    ErReportRef, ErSnapshot, ErSnapshotEntry, ErSources, ErTop, ErTopRef, ErTree, ErTreeContextExt,
    IntoErPart, IntoErTree, Layout,
};
#[cfg(feature = "src_locations")]
use alloc::string::ToString;
use alloc::{boxed::Box, string::String, vec::Vec};
#[cfg(feature = "src_locations")]
use core::panic::Location;
use core::{error::Error, fmt};

impl<E: Error + 'static> ErTree<E> {
    /// Put existing errors below this one, even if the list is empty.
    #[cfg_attr(feature = "src_locations", track_caller)]
    pub fn new(error: E, nodes: impl IntoIterator<Item = impl IntoErPart>) -> Self {
        let nodes = nodes.into_iter();
        let mut tree = Self::from(error);
        tree.nodes.reserve_exact(nodes.size_hint().0);
        #[cfg(feature = "stack_traces")]
        let (mut counted, mut next_index) = (0, 1);

        for node in nodes {
            let part = IntoErPart::into_er_part(node);
            #[cfg(feature = "stack_traces")]
            {
                let mut traces = part.stack_traces;
                if !traces.is_empty() {
                    for node in &tree.nodes[counted..] {
                        next_index += 1 + node.er_descendants().count();
                    }
                    counted = tree.nodes.len();
                    for trace in &mut traces {
                        trace.error_index.0 += next_index;
                    }
                }
                append_traces(&mut tree.stack_traces, traces);
            }
            tree.nodes.push(part.node);
        }

        tree
    }

    /// Add your error on the top, move everything else below it.
    /// |t| is the tree. The error is `t.top`.
    /// Use this when the new error needs something from the old one.
    /// Otherwise use `.er()`.
    ///
    /// `let error = error.er_with(|t| AnalyzeErr::new(t.top.code));`
    #[cfg_attr(feature = "src_locations", track_caller)]
    pub fn er_with<A>(self, top: impl FnOnce(&Self) -> A) -> ErTree<A>
    where
        E: Send + Sync,
        A: Error + 'static,
    {
        ErTree::from(top(&self)).with_part(self.into_er_part())
    }

    /// Erase the top error. This drops its stack traces, use `into_er_part()` to keep them.
    pub fn into_er_node(self) -> ErNode
    where
        E: Send + Sync,
    {
        self.into_er_part().node
    }

    pub fn into_er_part(self) -> ErPart
    where
        E: Send + Sync,
    {
        let error = Box::new(self.top);

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
        let mut entries = Vec::new();

        for entry in self.er_entries() {
            let mut message = String::new();
            let result = fmt::write(&mut message, format_args!("{}", entry.error));
            if result.is_err() {
                message.clear();
                message.push_str("ER_FMT_FAILED");
            }

            #[cfg(feature = "src_locations")]
            let src_location = entry.src_location.map(|location| {
                let file = location.file().to_string();
                let line = location.line();
                let column = location.column();

                ErSnapshotLocation { file, line, column }
            });

            entries.push(ErSnapshotEntry {
                message,
                kind: entry.kind,
                index: entry.index,
                parent: entry.parent,
                depth: entry.depth,
                is_last: entry.is_last,
                source_truncated: entry.source_truncated,
                #[cfg(feature = "src_locations")]
                src_location,
            });
        }

        ErSnapshot { entries }
    }
}
impl<E: Error + Send + Sync + 'static, Mode> ErTreeContextExt<Mode> for ErTree<E> {
    #[cfg_attr(feature = "src_locations", track_caller)]
    fn er<A>(self, top: impl ErMake<A, Mode>) -> ErTree<A>
    where
        A: Error + 'static,
    {
        ErTree::from(top.er_make()).with_part(self.into_er_part())
    }
}
impl<E> ErTree<E> {
    pub fn with_part(mut self, part: ErPart) -> Self {
        self.nodes.reserve_exact(1);
        self.push_part(part);
        self
    }

    pub fn push_part(&mut self, part: ErPart) {
        #[cfg(feature = "stack_traces")]
        {
            let mut traces = part.stack_traces;
            if !traces.is_empty() {
                let offset = 1 + self.er_descendants().count();
                for trace in &mut traces {
                    trace.error_index.0 += offset;
                }
            }
            append_traces(&mut self.stack_traces, traces);
        }
        self.nodes.push(part.node);
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
impl<E> IntoErTree for ErTree<E> {
    type Error = E;

    fn into_er_tree(self) -> Self {
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
impl<E: Error + Send + Sync + 'static> IntoErPart for ErTree<E> {
    fn into_er_part(self) -> ErPart {
        ErTree::into_er_part(self)
    }
}
