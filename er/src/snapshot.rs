use crate::{types::Layout, walk::ErEntryKind};
use alloc::{string::String, vec::Vec};

/// Saved messages and locations, no original error values.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[must_use]
pub struct ErSnapshot {
    /// Tree order, with indices linking each error to the one above it.
    /// Don't reorder these unless you also fix everything that says where they belong in the tree.
    pub entries: Vec<ErSnapshotEntry>,
}

/// One saved error, not one printed line.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ErSnapshotEntry {
    pub message: String,
    pub kind: ErEntryKind,
    pub index: usize,
    pub parent: Option<usize>,
    pub depth: usize,
    pub is_last: bool,
    /// The native source walk stopped here, the saved tree is incomplete.
    pub source_truncated: bool,
    #[cfg(feature = "src_locations")]
    #[cfg_attr(feature = "serde", serde(default))]
    pub src_location: Option<ErSnapshotLocation>,
}

/// File, line, column.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ErSnapshotLocation {
    pub file: String,
    pub line: u32,
    pub column: u32,
}

/// The whole saved report, borrows the snapshot.
#[derive(Clone, Copy)]
#[must_use]
pub struct ErSnapshotReport<'a> {
    pub snapshot: &'a ErSnapshot,
    pub layout: Layout,
}

/// Just the saved outer error, borrows the snapshot.
#[derive(Clone, Copy)]
#[must_use]
pub struct ErSnapshotTop<'a> {
    pub snapshot: &'a ErSnapshot,
    pub layout: Layout,
}
