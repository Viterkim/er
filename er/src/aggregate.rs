#[cfg(feature = "stack_traces")]
use crate::ErStackTrace;
#[cfg(feature = "stack_traces")]
use crate::impls::stack_trace::append_traces;
use crate::{ErInput, ErMake, ErNode, ErResult, ErTree};
use alloc::vec::Vec;
use core::error::Error;

#[doc(hidden)]
pub struct CollectedErrors {
    pub nodes: Vec<ErNode>,
    #[cfg(feature = "stack_traces")]
    pub stack_traces: Vec<ErStackTrace>,
}
impl CollectedErrors {
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    #[track_caller]
    pub fn into_tree<A: Error + 'static>(self, top: A) -> ErTree<A> {
        ErTree {
            nodes: self.nodes,
            #[cfg(feature = "stack_traces")]
            stack_traces: self.stack_traces,
            ..ErTree::from(top)
        }
    }

    #[track_caller]
    pub fn finish<A, Mode>(self, top: impl ErMake<A, Mode>) -> ErResult<(), A>
    where
        A: Error + 'static,
    {
        if self.is_empty() {
            Ok(())
        } else {
            Err(self.into_tree(top.er_make()))
        }
    }
}

#[doc(hidden)]
#[track_caller]
pub fn gather<E, T, Input>(results: impl IntoIterator<Item = Result<T, E>>) -> CollectedErrors
where
    E: ErInput<Input>,
{
    let mut nodes: Vec<ErNode> = Vec::new();
    #[cfg(feature = "stack_traces")]
    let mut traces = Vec::new();
    #[cfg(feature = "stack_traces")]
    let (mut counted, mut next_index) = (0, 1);

    for result in results {
        match result {
            Ok(value) => drop(value),
            Err(error) => {
                let part = ErInput::into_er_input(error);
                #[cfg(feature = "stack_traces")]
                {
                    let mut incoming = part.stack_traces;
                    if !incoming.is_empty() {
                        for node in &nodes[counted..] {
                            next_index += 1 + node.er_descendants().count();
                        }
                        counted = nodes.len();
                        for trace in &mut incoming {
                            trace.error_index.0 += next_index;
                        }
                    }
                    append_traces(&mut traces, incoming);
                }
                nodes.push(part.node);
            }
        }
    }

    CollectedErrors {
        nodes,
        #[cfg(feature = "stack_traces")]
        stack_traces: traces,
    }
}

/// The collection loop behind `er_all!`.
#[track_caller]
pub fn collect<A, E, T, Input, Mode>(
    top: impl ErMake<A, Mode>,
    results: impl IntoIterator<Item = Result<T, E>>,
) -> ErResult<(), A>
where
    A: Error + 'static,
    E: ErInput<Input>,
{
    gather(results).finish(top)
}
