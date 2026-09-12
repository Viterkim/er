use crate::{BoxError, ErEntries, ErNode, ErNodes, ErSources, IntoErNode};
use alloc::vec::Vec;
#[cfg(feature = "src_locations")]
use core::panic::Location;
use core::{error::Error, mem::take};

impl ErNode {
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

        self.er_descendants().find_map(Self::er_find_here::<T>)
    }

    /// Finds all the instances of an error type, for when you have duplicates.
    pub fn er_find_all<T: Error + 'static>(&self) -> impl Iterator<Item = &T> {
        ErEntries::new(
            &*self.error,
            &self.nodes,
            #[cfg(feature = "src_locations")]
            Some(self.src_location),
            #[cfg(not(feature = "src_locations"))]
            None,
        )
        .filter_map(|entry| entry.error.downcast_ref::<T>())
    }

    /// Searches this error and its native sources, but not stored children.
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

impl<E: Into<BoxError>> IntoErNode for E {
    #[cfg_attr(feature = "src_locations", track_caller)]
    fn into_er_node(self) -> ErNode {
        let error = self.into();
        let nodes = Vec::new();

        ErNode {
            error,
            nodes,
            #[cfg(feature = "src_locations")]
            src_location: Location::caller(),
        }
    }
}
impl IntoErNode for ErNode {
    fn into_er_node(self) -> Self {
        self
    }
}
