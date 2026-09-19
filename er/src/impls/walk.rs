use crate::walk::{MAX_SOURCE_HOPS, Pending};
use crate::{ErEntries, ErEntry, ErEntryKind, ErFindAll, ErNode, ErNodes, ErSources, SrcLocation};
use alloc::{vec, vec::Vec};
use core::error::Error;

impl<'a> ErEntries<'a> {
    #[inline]
    pub fn new(
        error: &'a (dyn Error + 'static),
        nodes: &'a [ErNode],
        src_location: Option<SrcLocation>,
    ) -> Self {
        #[cfg(not(feature = "src_locations"))]
        let _ = src_location;

        let entry = ErEntry {
            error,
            kind: ErEntryKind::Root,
            index: 0,
            parent: None,
            depth: 0,
            is_last: true,
            source_truncated: false,
            #[cfg(feature = "src_locations")]
            src_location,
        };
        let pending = vec![Pending {
            entry,
            nodes,
            sources_left: MAX_SOURCE_HOPS,
        }];

        Self {
            pending,
            next_index: 0,
        }
    }
}
impl<'a> Iterator for ErEntries<'a> {
    type Item = ErEntry<'a>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let Pending {
            mut entry,
            nodes,
            sources_left,
        } = self.pending.pop()?;
        entry.index = self.next_index;
        self.next_index += 1;

        for (index, node) in nodes.iter().enumerate().rev() {
            let child = ErEntry {
                error: &*node.error,
                kind: ErEntryKind::Node,
                index: 0,
                parent: Some(entry.index),
                depth: entry.depth + 1,
                is_last: index + 1 == nodes.len(),
                source_truncated: false,
                #[cfg(feature = "src_locations")]
                src_location: Some(node.src_location),
            };

            self.pending.push(Pending {
                entry: child,
                nodes: &node.nodes,
                sources_left: MAX_SOURCE_HOPS,
            });
        }

        if let Some(source) = entry.error.source() {
            if sources_left == 0 {
                entry.source_truncated = true;
                return Some(entry);
            }

            let child = ErEntry {
                error: source,
                kind: ErEntryKind::Source,
                index: 0,
                parent: Some(entry.index),
                depth: entry.depth + 1,
                is_last: nodes.is_empty(),
                source_truncated: false,
                #[cfg(feature = "src_locations")]
                src_location: None,
            };

            self.pending.push(Pending {
                entry: child,
                nodes: &[],
                sources_left: sources_left - 1,
            });
        }

        Some(entry)
    }
}

impl<'a> ErNodes<'a> {
    #[inline]
    pub fn new(roots: &'a [ErNode]) -> Self {
        let pending = roots.iter().rev().collect();
        Self { pending }
    }
}
impl<'a> Iterator for ErNodes<'a> {
    type Item = &'a ErNode;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let node = self.pending.pop()?;
        self.pending.extend(node.nodes.iter().rev());
        Some(node)
    }
}

impl<'a> ErSources<'a> {
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
    pub fn new(error: &'a (dyn Error + 'static), nodes: &'a [ErNode]) -> Self {
        Self {
            error: Some(error),
            yielded_error: false,
            sources_left: MAX_SOURCE_HOPS,
            nodes,
            pending: Vec::new(),
            marker: core::marker::PhantomData,
        }
    }

    pub fn next_node(&mut self) -> Option<&'a ErNode> {
        loop {
            if let Some((node, rest)) = self.nodes.split_first() {
                if node.nodes.is_empty() {
                    self.nodes = rest;
                } else {
                    if !rest.is_empty() {
                        self.pending.push(rest);
                    }
                    self.nodes = &node.nodes;
                }
                return Some(node);
            }

            self.nodes = self.pending.pop()?;
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

            let node = self.next_node()?;
            self.error = Some(&*node.error);
            self.yielded_error = false;
            self.sources_left = MAX_SOURCE_HOPS;
        }
    }
}
