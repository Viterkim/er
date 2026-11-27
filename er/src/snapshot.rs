use crate::{ErSavedList, walk::ErEntryKind};
use alloc::{borrow::Cow, string::String, vec::Vec};

/// A copy of the tree's messages and locations that outlives the original errors.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
#[must_use]
pub struct ErSnapshot {
    /// Tree order, with parent indices linking each error to the one above it.
    pub entries: Vec<ErSnapshotEntry>,
    /// Reports with no owning row, such as a save on an empty snapshot.
    pub unattached: ErSavedList,
}

/// One saved error, not one printed line.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ErSnapshotEntry {
    pub message: String,
    /// The attachments shown on this row, in report order.
    pub saved: ErSavedList,
    pub kind: ErEntryKind,
    pub index: usize,
    pub parent: Option<usize>,
    pub depth: usize,
    pub is_last: bool,
    pub source_truncated: bool,
    pub src_location: Option<ErSnapshotLocation>,
}

/// File, line, column.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ErSnapshotLocation {
    pub file: Cow<'static, str>,
    pub line: u32,
    pub column: u32,
}
