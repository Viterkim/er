use crate::{BoxError, ErFindAll, ErNode, ErNodes, ErPart, ErSources, IntoErPart};
use alloc::vec::Vec;
#[cfg(feature = "src_locations")]
use core::panic::Location;
use core::{error::Error, mem::take};

impl ErNode {
    #[inline]
    pub fn er_sources(&self) -> ErSources<'_> {
        ErSources::new(self.error.source())
    }

    /// Every node below this one.
    pub fn er_descendants(&self) -> ErNodes<'_> {
        ErNodes::new(&self.nodes)
    }

    /// Returns the FIRST match.
    pub fn er_find<T: Error + 'static>(&self) -> Option<&T> {
        if let Some(found) = self.er_find_here::<T>() {
            return Some(found);
        }

        // The usual Er tree is one error under another (walk without a traversal vec)
        let mut nodes = self.nodes.as_slice();
        loop {
            match nodes {
                [] => return None,
                [node] => {
                    if let Some(found) = node.er_find_here::<T>() {
                        return Some(found);
                    }
                    nodes = &node.nodes;
                }
                nodes => return ErNodes::new(nodes).find_map(Self::er_find_here::<T>),
            }
        }
    }

    /// Finds all the instances of an error type, for when you have duplicates.
    pub fn er_find_all<T: Error + 'static>(&self) -> ErFindAll<'_, T> {
        ErFindAll::new(&*self.error, &self.nodes)
    }

    /// Searches this error and its native sources, but not stored sub errors.
    pub fn er_find_here<T: Error + 'static>(&self) -> Option<&T> {
        let error: &(dyn Error + 'static) = &*self.error;

        error
            .downcast_ref::<T>()
            .or_else(|| self.er_sources().find_map(<dyn Error>::downcast_ref::<T>))
    }
}
impl Drop for ErNode {
    fn drop(&mut self) {
        let mut pending = take(&mut self.nodes);

        while let Some(mut node) = pending.pop() {
            pending.append(&mut node.nodes);
        }
    }
}
impl IntoErPart for ErNode {
    fn into_er_part(self) -> ErPart {
        ErPart {
            node: self,
            #[cfg(feature = "stack_traces")]
            stack_traces: Vec::new(),
        }
    }
}
impl IntoErPart for ErPart {
    fn into_er_part(self) -> Self {
        self
    }
}

impl<E: Into<BoxError>> IntoErPart for E {
    #[cfg_attr(feature = "src_locations", track_caller)]
    fn into_er_part(self) -> ErPart {
        let error = self.into();
        let nodes = Vec::new();

        ErPart {
            node: ErNode {
                error,
                nodes,
                #[cfg(feature = "src_locations")]
                src_location: Location::caller(),
            },
            #[cfg(feature = "stack_traces")]
            stack_traces: Vec::new(),
        }
    }
}
