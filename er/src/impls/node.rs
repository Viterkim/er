#[cfg(er_unsync)]
use crate::ErBoxedInput;
#[cfg(any(er_unsync, target_has_atomic = "ptr"))]
use crate::impls::shared_tree::ErSharedNode;
use crate::{
    BoxError, ErFindAll, ErInput, ErNode, ErNodes, ErPart, ErSavedList, ErSources, IntoErPart,
};
#[cfg(er_unsync)]
use alloc::boxed::Box;
use alloc::vec::Vec;
#[cfg(feature = "src_locations")]
use core::panic::Location;
use core::{error::Error, mem::take, ops::Deref};

pub(crate) type StoredError = <BoxError as Deref>::Target;

impl ErNode {
    /// This node's error and sub errors. Use it when you walk nodes yourself, it also looks inside shared trees.
    #[inline]
    pub fn er_view(&self) -> (&(dyn Error + 'static), &[ErNode]) {
        let (error, nodes) = self.stored_view();
        (error, nodes)
    }

    #[inline]
    fn stored_view(&self) -> (&StoredError, &[ErNode]) {
        #[cfg(any(er_unsync, target_has_atomic = "ptr"))]
        if self.nodes.is_empty()
            && let Some(shared) = self.error.downcast_ref::<ErSharedNode>()
            && let Some(view) = shared.view()
        {
            return view;
        }

        (&*self.error, &self.nodes)
    }

    /// This error's native `source()` chain.
    #[inline]
    pub fn er_sources(&self) -> ErSources<'_> {
        ErSources::new(self.error.source())
    }

    /// Every node below this one.
    pub fn er_descendants(&self) -> ErNodes<'_> {
        ErNodes::new(self.er_view().1)
    }

    /// Returns the FIRST match.
    pub fn er_find<T: Error + 'static>(&self) -> Option<&T> {
        let (error, nodes) = self.er_view();

        find_here::<T>(error).or_else(|| find_below(nodes))
    }

    /// Finds all the instances of an error type, for when you have duplicates.
    pub fn er_find_all<T: Error + 'static>(&self) -> ErFindAll<'_, T> {
        let (error, nodes) = self.er_view();
        ErFindAll::new(error, nodes)
    }

    /// True if this type is in this node, its sub errors or native sources.
    pub fn er_contains<T: Error + 'static>(&self) -> bool {
        self.er_find::<T>().is_some()
    }

    /// Searches this error and its native sources, but not stored sub errors.
    pub fn er_find_here<T: Error + 'static>(&self) -> Option<&T> {
        find_here(self.er_view().0)
    }
}

/// Which shared tree this node holds, if it holds one.
pub fn shared_address(node: &ErNode) -> Option<usize> {
    #[cfg(any(er_unsync, target_has_atomic = "ptr"))]
    if node.nodes.is_empty() {
        return node.error.downcast_ref::<ErSharedNode>()?.address();
    }

    let _ = node;
    None
}

/// The first `T` in these nodes or below them.
pub fn find_below<T: Error + 'static>(mut nodes: &[ErNode]) -> Option<&T> {
    // The usual Er tree is one error under another (walk without a traversal vec)
    loop {
        match nodes {
            [] => return None,
            [node] => {
                let (error, sub_nodes) = node.er_view();
                if let Some(found) = find_here::<T>(error) {
                    return Some(found);
                }
                nodes = sub_nodes;
            }
            nodes => return ErFindAll::first(nodes).next(),
        }
    }
}

fn find_here<'a, T: Error + 'static>(error: &'a (dyn Error + 'static)) -> Option<&'a T> {
    error
        .downcast_ref::<T>()
        .or_else(|| ErSources::new(error.source()).find_map(<dyn Error>::downcast_ref::<T>))
}
impl Drop for ErNode {
    #[inline(always)]
    fn drop(&mut self) {
        if !self.nodes.is_empty() {
            drop_nodes(take(&mut self.nodes));
        }
    }
}

fn drop_nodes(mut pending: Vec<ErNode>) {
    while let [only] = pending.as_mut_slice() {
        pending = take(&mut only.nodes);
    }

    while let Some(mut node) = pending.pop() {
        append_nodes(&mut pending, &mut node.nodes);
        drop(node);

        // Drop the leaves in place when several errors share a parent.
        while pending.last().is_some_and(|node| node.nodes.is_empty()) {
            pending.pop();
        }
    }
}

#[inline]
pub(crate) fn append_nodes(pending: &mut Vec<ErNode>, nodes: &mut Vec<ErNode>) {
    if nodes.is_empty() {
        return;
    }

    if pending.is_empty() && nodes.len() > pending.capacity() {
        core::mem::swap(pending, nodes);
    } else {
        pending.append(nodes);
    }
}

impl IntoErPart for ErNode {
    type Error = <BoxError as Deref>::Target;

    fn er_error(&self) -> &Self::Error {
        self.stored_view().0
    }

    fn into_er_part(self) -> ErPart {
        ErPart {
            node: self,
            saved: ErSavedList::default(),
        }
    }
}
impl IntoErPart for ErPart {
    type Error = <BoxError as Deref>::Target;

    fn er_error(&self) -> &Self::Error {
        self.node.er_error()
    }

    fn into_er_part(self) -> Self {
        self
    }
}

impl<E: IntoErPart<Input>, Input> ErInput<Input> for E {
    type Error = E::Error;

    fn er_input_error(&self) -> &Self::Error {
        self.er_error()
    }

    #[cfg_attr(feature = "src_locations", track_caller)]
    fn into_er_input(self) -> ErPart {
        self.into_er_part()
    }
}

// These boxes coerce to BoxError, std just doesn't give them an Into impl.
#[cfg(er_unsync)]
macro_rules! boxed_input {
    ($($bounds:tt)*) => {
        impl IntoErPart<ErBoxedInput> for Box<dyn Error $($bounds)*> {
            type Error = Self;

            fn er_error(&self) -> &Self {
                self
            }

            #[cfg_attr(feature = "src_locations", track_caller)]
            fn into_er_part(self) -> ErPart {
                let error: BoxError = self;
                error.into_er_part()
            }
        }
    };
}
#[cfg(er_unsync)]
boxed_input!(+ Send + Sync);
#[cfg(er_unsync)]
boxed_input!(+ Send);
#[cfg(er_unsync)]
boxed_input!(+ Sync);

impl<E: Into<BoxError>> IntoErPart for E {
    type Error = E;

    fn er_error(&self) -> &E {
        self
    }

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
            saved: ErSavedList::default(),
        }
    }
}
