use crate::{ErSaved, walk::ErEntryKind};
use alloc::{borrow::Cow, string::String, vec::Vec};

/// A copy of the tree's messages and locations that outlives the original errors.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[must_use]
pub struct ErSnapshot {
    /// Tree order, with parent indices linking each error to the one above it.
    pub entries: Vec<ErSnapshotEntry>,
    /// The tree's saved reports/traces
    pub saved: Vec<ErSaved>,
}

/// One saved error, not one printed line.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ErSnapshotEntry {
    pub message: String,
    pub kind: ErEntryKind,
    pub index: usize,
    pub parent: Option<usize>,
    pub depth: usize,
    pub is_last: bool,
    pub source_truncated: bool,
    #[cfg(feature = "src_locations")]
    pub src_location: Option<ErSnapshotLocation>,
}

/// File, line, column.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ErSnapshotLocation {
    pub file: Cow<'static, str>,
    pub line: u32,
    pub column: u32,
}
