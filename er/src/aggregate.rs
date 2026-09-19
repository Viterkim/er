use crate::{Er, ErMake, ErTree, IntoErNode};
use alloc::vec::Vec;
use core::error::Error;

/// The collection loop behind `er_all!`.
#[cfg_attr(feature = "src_locations", track_caller)]
pub fn collect<A, Mode, Parent, E, T>(
    parent: impl FnOnce() -> Parent,
    results: impl IntoIterator<Item = Result<T, E>>,
) -> Er<(), A>
where
    A: Error + 'static,
    Parent: ErMake<A, Mode>,
    E: IntoErNode,
{
    let mut nodes = Vec::new();
    for result in results {
        match result {
            Ok(value) => drop(value),
            Err(error) => nodes.push(IntoErNode::into_er_node(error)),
        }
    }

    if nodes.is_empty() {
        Ok(())
    } else {
        Err(ErTree {
            nodes,
            ..ErTree::from(parent().er_make())
        })
    }
}
