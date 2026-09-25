#[cfg(feature = "stack_traces")]
use crate::impls::stack_trace::append_traces;
use crate::{Er, ErNode, ErTree, IntoErPart};
use alloc::vec::Vec;
use core::error::Error;

/// The collection loop behind `er_all!`.
#[cfg_attr(feature = "src_locations", track_caller)]
pub fn collect<A, E, T>(
    top: impl FnOnce() -> A,
    results: impl IntoIterator<Item = Result<T, E>>,
) -> Er<(), A>
where
    A: Error + 'static,
    E: IntoErPart,
{
    let mut nodes: Vec<ErNode> = Vec::new();
    #[cfg(feature = "stack_traces")]
    let mut traces = Vec::new();
    #[cfg(feature = "stack_traces")]
    let (mut counted, mut next_id) = (0, 1);

    for result in results {
        match result {
            Ok(value) => drop(value),
            Err(error) => {
                let part = IntoErPart::into_er_part(error);
                #[cfg(feature = "stack_traces")]
                {
                    let mut incoming = part.stack_traces;
                    if !incoming.is_empty() {
                        for node in &nodes[counted..] {
                            next_id += 1 + node.er_descendants().count();
                        }
                        counted = nodes.len();
                        for trace in &mut incoming {
                            trace.error_id.0 += next_id;
                        }
                    }
                    append_traces(&mut traces, incoming);
                }
                nodes.push(part.node);
            }
        }
    }

    if nodes.is_empty() {
        Ok(())
    } else {
        Err(ErTree {
            nodes,
            #[cfg(feature = "stack_traces")]
            stack_traces: traces,
            ..ErTree::from(top())
        })
    }
}
