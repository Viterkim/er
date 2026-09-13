pub use crate::types::{ErEntries, ErNodes, ErSources};
pub use crate::types_helpers::Pending;

/// Native source hops per root or stored node. Doesn't limit Er tree depth.
pub const MAX_SOURCE_HOPS: usize = 256;
