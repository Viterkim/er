#[cfg(feature = "src_locations")]
use crate::ErSnapshotLocation;
use crate::{
    ErEntries, ErEntry, ErFindAll, ErMake, ErNode, ErNodes, ErPayload, ErReport, ErReportRef,
    ErSnapshot, ErSnapshotEntry, ErSources, ErTop, ErTopRef, ErTree, ErTreeContext, IntoErNode,
    IntoErTree, Layout,
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
    pub fn new(error: E, nodes: impl IntoIterator<Item = impl IntoErNode>) -> Self {
        let nodes = nodes.into_iter();
        let mut collected = Vec::with_capacity(nodes.size_hint().0);
        for node in nodes {
            collected.push(IntoErNode::into_er_node(node));
        }

        Self {
            top: error,
            nodes: collected,
            #[cfg(feature = "src_locations")]
            src_location: Location::caller(),
        }
    }

    /// Add your error on the top, move everything else below it.
    /// |t| is the tree. The error is `t.top`.
    /// Use this when the new error needs something from the old one.
    /// Otherwise use `.er()`.
    ///
    /// `let error = error.er_with(|t| t.top.code);`
    #[cfg_attr(feature = "src_locations", track_caller)]
    pub fn er_with<A, P, Mode>(self, top: impl FnOnce(&Self) -> P) -> ErTree<A>
    where
        E: Send + Sync,
        A: Error + 'static,
        P: ErPayload<A, Mode>,
    {
        let top = top(&self).er_payload();
        ErTree::new(top, [self])
    }

    pub fn into_er_node(self) -> ErNode
    where
        E: Send + Sync,
    {
        let error = Box::new(self.top);

        ErNode {
            error,
            nodes: self.nodes,
            #[cfg(feature = "src_locations")]
            src_location: self.src_location,
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
impl<E: Error + Send + Sync + 'static, Mode> ErTreeContext<Mode> for ErTree<E> {
    #[cfg_attr(feature = "src_locations", track_caller)]
    fn er<A>(self, top: impl ErMake<A, Mode>) -> ErTree<A>
    where
        A: Error + 'static,
    {
        ErTree::new(top.er_make(), [self])
    }
}
impl<E> ErTree<E> {
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
            #[cfg(feature = "src_locations")]
            src_location: Location::caller(),
        }
    }
}
impl<E: Error + Send + Sync + 'static> IntoErNode for ErTree<E> {
    fn into_er_node(self) -> ErNode {
        ErTree::into_er_node(self)
    }
}
