use crate::{ErSaved, walk::ErEntryKind};
use alloc::{borrow::Cow, string::String, vec::Vec};

/// A copy of the tree's messages and locations that outlives the original errors.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[must_use]
pub struct ErSnapshot {
    /// Tree order, with indices linking each error to the one above it.
    /// Don't reorder these unless you also fix everything that says where they belong in the tree.
    pub entries: Vec<ErSnapshotEntry>,
    /// The tree's saved reports. Stack traces are stored as text, and so is a snapshot saved inside a saved snapshot.
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
    /// Live snapshots borrow the source filename. Use `path.into()` for a filename you own.
    pub file: Cow<'static, str>,
    pub line: u32,
    pub column: u32,
}
