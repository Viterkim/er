use crate::{ErEntryKind, ErSnapshot, ErSnapshotEntry, ErSnapshotReport, ErSnapshotTop, Layout};
use core::slice;

impl ErSnapshot {
    pub fn er_entries(&self) -> slice::Iter<'_, ErSnapshotEntry> {
        self.entries.iter()
    }

    pub fn er_for_each_entry<'a>(&'a self, visit: impl FnMut(&'a ErSnapshotEntry)) {
        self.er_entries().for_each(visit);
    }

    /// Stored child errors, skips the root and native sources.
    pub fn er_descendants(&self) -> impl Iterator<Item = &ErSnapshotEntry> {
        self.er_entries()
            .filter(|entry| entry.kind == ErEntryKind::Node)
    }

    pub const fn er_report(&self) -> ErSnapshotReport<'_> {
        ErSnapshotReport {
            snapshot: self,
            layout: Layout::Multiline,
        }
    }

    pub const fn er_top(&self) -> ErSnapshotTop<'_> {
        ErSnapshotTop {
            snapshot: self,
            layout: Layout::Multiline,
        }
    }
}
