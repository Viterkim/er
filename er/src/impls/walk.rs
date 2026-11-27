use crate::impls::node::shared_address;
use crate::walk::{MAX_SOURCE_HOPS, PendingError, PendingNodes};
use crate::{
    ErEntries, ErEntry, ErEntryKind, ErFindAll, ErNode, ErNodes, ErSaved, ErSavedList, ErSources,
    ErSrcLocation,
};
use alloc::{collections::BTreeSet, vec::Vec};
use core::error::Error;

impl<'a> ErEntry<'a> {
    /// The reports saved on this error, including a shared tree's top.
    pub fn er_saved(&self) -> impl Iterator<Item = &'a ErSaved> + use<'a> {
        let (shared, own) = self.saved_slices();
        shared.iter().chain(own)
    }

    pub(crate) fn saved_slices(&self) -> (&'a [ErSaved], &'a [ErSaved]) {
        (
            self.shared_saved.map_or(&[], core::ops::Deref::deref),
            self.saved.map_or(&[], core::ops::Deref::deref),
        )
    }
}

impl<'a> ErEntries<'a> {
    /// Walk this root and the errors below it, including native sources.
    #[inline]
    pub fn new(
        error: &'a (dyn Error + 'static),
        nodes: &'a [ErNode],
        src_location: Option<ErSrcLocation>,
    ) -> Self {
        let next_error = Some(PendingError {
            error,
            depth: 0,
            sources_left: MAX_SOURCE_HOPS,
            is_last: true,
            saved: None,
            src_location,
        });
        let nodes = PendingNodes {
            siblings: nodes,
            parent: 0,
            depth: 1,
        };

        Self {
            next_error,
            nodes,
            pending: Vec::new(),
            next_index: 0,
        }
    }

    #[inline]
    pub(crate) fn with_saved(
        error: &'a (dyn Error + 'static),
        nodes: &'a [ErNode],
        src_location: Option<ErSrcLocation>,
        saved: &'a ErSavedList,
    ) -> Self {
        let mut entries = Self::new(error, nodes, src_location);
        if let Some(root) = entries.next_error.as_mut()
            && saved.0.is_some()
        {
            root.saved = Some(saved);
        }
        entries
    }

    #[inline(always)]
    fn next_node(&mut self, index: usize) -> Option<(ErEntry<'a>, bool)> {
        if self.nodes.siblings.is_empty() {
            self.nodes = self.pending.pop()?;
        }

        let (node, rest) = self.nodes.siblings.split_first()?;
        let (error, sub_errors, shared_saved) = node.stored_view();
        let entry = ErEntry {
            error,
            kind: ErEntryKind::Node,
            index,
            parent: Some(self.nodes.parent),
            depth: self.nodes.depth,
            is_last: rest.is_empty(),
            source_truncated: false,
            saved: node.saved.0.as_ref().map(|_| &node.saved),
            shared_saved: shared_saved.filter(|saved| saved.0.is_some()),
            src_location: node.src_location,
        };

        self.nodes.siblings = rest;
        if !sub_errors.is_empty() {
            if !rest.is_empty() {
                self.pending.push(self.nodes);
            }

            self.nodes = PendingNodes {
                siblings: sub_errors,
                parent: index,
                depth: entry.depth + 1,
            };
        }

        Some((entry, !sub_errors.is_empty()))
    }
}
impl<'a> Iterator for ErEntries<'a> {
    type Item = ErEntry<'a>;

    fn size_hint(&self) -> (usize, Option<usize>) {
        let known = self.nodes.siblings.len() + usize::from(self.next_error.is_some());
        (known, (known == 0 && self.pending.is_empty()).then_some(0))
    }

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        let index = self.next_index;
        let (mut entry, has_sub_errors, sources_left) = if let Some(next) = self.next_error.take() {
            let root = index == 0;
            let entry = ErEntry {
                error: next.error,
                kind: if root {
                    ErEntryKind::Root
                } else {
                    ErEntryKind::Source
                },
                index,
                parent: index.checked_sub(1),
                depth: next.depth,
                is_last: next.is_last,
                source_truncated: false,
                saved: next.saved,
                shared_saved: None,
                src_location: next.src_location,
            };

            (
                entry,
                root && !self.nodes.siblings.is_empty(),
                next.sources_left,
            )
        } else {
            let (entry, has_sub_errors) = self.next_node(index)?;
            (entry, has_sub_errors, MAX_SOURCE_HOPS)
        };

        self.next_index += 1;
        if let Some(error) = entry.error.source() {
            if sources_left == 0 {
                entry.source_truncated = true;
            } else {
                self.next_error = Some(PendingError {
                    error,
                    depth: entry.depth + 1,
                    sources_left: sources_left - 1,
                    is_last: !has_sub_errors,
                    saved: None,
                    src_location: None,
                });
            }
        }

        Some(entry)
    }
}

impl<'a> ErNodes<'a> {
    /// Walk these nodes and everything below them.
    #[inline]
    pub fn new(nodes: &'a [ErNode]) -> Self {
        Self {
            nodes: nodes.iter(),
            pending: Vec::new(),
            seen: None,
        }
    }

    /// Like `new`, but a shared tree that sits under several errors only gets walked the first time. The node
    /// holding it still shows up every time. A search for one match finds nothing new in there, and going in
    /// again through every path doubles the work per layer of sharing.
    #[inline]
    pub fn once(nodes: &'a [ErNode]) -> Self {
        Self {
            seen: Some(BTreeSet::new()),
            ..Self::new(nodes)
        }
    }

    #[inline]
    fn walks_into(&mut self, node: &ErNode) -> bool {
        let Some(seen) = &mut self.seen else {
            return true;
        };

        shared_address(node).is_none_or(|address| seen.insert(address))
    }

    #[inline(always)]
    fn next_with<T>(
        &mut self,
        select: impl FnOnce(&'a ErNode, &'a (dyn Error + 'static), Option<&'a ErSavedList>) -> T,
    ) -> Option<T> {
        let node = match self.nodes.next() {
            Some(node) => node,
            None => {
                self.nodes = self.pending.pop()?;
                self.nodes.next()?
            }
        };

        let (error, nodes, shared_saved) = node.stored_view();
        if !nodes.is_empty() && self.walks_into(node) {
            let rest = core::mem::replace(&mut self.nodes, nodes.iter());
            if !rest.as_slice().is_empty() {
                self.pending.push(rest);
            }
        }

        Some(select(node, error, shared_saved))
    }

    pub(crate) fn saved(mut self) -> impl Iterator<Item = &'a ErSaved> {
        core::iter::from_fn(move || {
            self.next_with(|node, _, shared| {
                let shared = shared.map_or(&[][..], core::ops::Deref::deref);
                shared.iter().chain(&node.saved)
            })
        })
        .flatten()
    }
}
impl<'a> Iterator for ErNodes<'a> {
    type Item = &'a ErNode;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.next_with(|node, _, _| node)
    }

    // The rest of the current siblings are coming for sure, enough for collect() to skip most regrowing.
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.nodes.len(), None)
    }
}

impl<'a> ErSources<'a> {
    /// Follow the native source chain, starting here.
    pub const fn new(next: Option<&'a (dyn Error + 'static)>) -> Self {
        Self {
            next,
            remaining: MAX_SOURCE_HOPS,
            truncated: false,
        }
    }
}
impl<'a> Iterator for ErSources<'a> {
    type Item = &'a (dyn Error + 'static);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let error = self.next.take()?;
        if self.remaining == 0 {
            self.truncated = true;
            return None;
        }

        self.remaining -= 1;
        self.next = error.source();
        Some(error)
    }
}

impl<'a, T> ErFindAll<'a, T> {
    /// Find this type in the root and below, including native sources.
    pub fn new(error: &'a (dyn Error + 'static), nodes: &'a [ErNode]) -> Self {
        Self {
            error: Some(error),
            yielded_error: false,
            sources_left: MAX_SOURCE_HOPS,
            nodes: ErNodes::new(nodes),
            marker: core::marker::PhantomData,
        }
    }

    /// Like `below` for a search that stops at the first match, see `ErNodes::once`.
    pub fn first(nodes: &'a [ErNode]) -> Self {
        Self {
            nodes: ErNodes::once(nodes),
            ..Self::below(nodes)
        }
    }

    /// Find this type in these nodes and below.
    pub fn below(nodes: &'a [ErNode]) -> Self {
        Self {
            error: None,
            yielded_error: true,
            sources_left: MAX_SOURCE_HOPS,
            nodes: ErNodes::new(nodes),
            marker: core::marker::PhantomData,
        }
    }

    pub fn next_source(
        &self,
        error: &'a (dyn Error + 'static),
    ) -> Option<&'a (dyn Error + 'static)> {
        if self.sources_left == 0 {
            None
        } else {
            error.source()
        }
    }
}
impl<'a, T: Error + 'static> Iterator for ErFindAll<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(error) = self.error {
                if !self.yielded_error {
                    self.yielded_error = true;
                    if let Some(found) = error.downcast_ref::<T>() {
                        return Some(found);
                    }
                }

                if let Some(source) = self.next_source(error) {
                    self.error = Some(source);
                    self.yielded_error = false;
                    self.sources_left -= 1;
                    continue;
                }

                self.error = None;
            }

            self.error = Some(self.nodes.next_with(|_, error, _| error)?);
            self.yielded_error = false;
            self.sources_left = MAX_SOURCE_HOPS;
        }
    }
}
