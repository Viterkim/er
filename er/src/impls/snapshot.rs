#[cfg(feature = "src_locations")]
use crate::ErSnapshotLocation;
use crate::lines;
use crate::render::{
    format_string,
    report::{valid_depth, write_entries},
    write_top,
};
use crate::{
    ErEntries, ErEntryKind, ErLineError, ErSnapshot, ErSnapshotEntry, ErSnapshotReport,
    ErSnapshotTop, Layout,
};
use alloc::{
    string::{String, ToString},
    vec::Vec,
};
use core::{error::Error, fmt, ops::Deref, slice};

impl ErSnapshot {
    /// The saved report as text.
    pub fn er_report_string(&self) -> String {
        self.er_report().to_string()
    }

    /// Just the saved outer error as text.
    pub fn er_top_string(&self) -> String {
        self.er_top().to_string()
    }

    /// The saved errors, in tree order.
    pub fn er_entries(&self) -> slice::Iter<'_, ErSnapshotEntry> {
        self.entries.iter()
    }

    /// Call once per saved error.
    pub fn er_for_each_entry<'a>(&'a self, visit: impl FnMut(&'a ErSnapshotEntry)) {
        self.er_entries().for_each(visit);
    }

    /// Stored sub errors, skips the root and native sources.
    pub fn er_descendants(&self) -> impl Iterator<Item = &ErSnapshotEntry> {
        self.er_entries()
            .filter(|entry| entry.kind == ErEntryKind::Node)
    }

    /// Print the whole saved report.
    pub const fn er_report(&self) -> ErSnapshotReport<'_> {
        ErSnapshotReport {
            snapshot: self,
            layout: Layout::Multiline,
        }
    }

    /// Print just the saved top error.
    pub const fn er_top(&self) -> ErSnapshotTop<'_> {
        ErSnapshotTop {
            snapshot: self,
            layout: Layout::Multiline,
        }
    }
}
impl From<ErEntries<'_>> for ErSnapshot {
    fn from(walk: ErEntries<'_>) -> Self {
        let mut entries = Vec::new();

        for entry in walk {
            let message = format_string(entry.error);

            #[cfg(feature = "src_locations")]
            let src_location = entry.src_location.map(|location| ErSnapshotLocation {
                file: location.file().to_string(),
                line: location.line(),
                column: location.column(),
            });

            entries.push(ErSnapshotEntry {
                message,
                kind: entry.kind,
                index: entry.index,
                parent: entry.parent,
                depth: entry.depth,
                is_last: entry.is_last,
                source_truncated: entry.source_truncated,
                #[cfg(feature = "src_locations")]
                src_location,
            });
        }

        Self { entries }
    }
}

impl ErSnapshotReport<'_> {
    /// The saved report as text, keeps this layout.
    pub fn er_report_string(&self) -> String {
        self.to_string()
    }

    /// Just the saved outer error as text, keeps this layout.
    pub fn er_top_string(&self) -> String {
        ErSnapshotTop {
            snapshot: self.snapshot,
            layout: self.layout,
        }
        .to_string()
    }

    /// Pick how it gets printed.
    pub const fn layout(self, layout: Layout) -> Self {
        Self { layout, ..self }
    }

    /// Print on one line.
    pub const fn single_line(self) -> Self {
        self.layout(Layout::SingleLine)
    }

    /// Only formats once, no line endings.
    pub fn for_each_line(&self, emit: impl FnMut(&str)) -> fmt::Result {
        lines::for_each_line(self, emit)
    }

    /// Like `for_each_line()`, but the callback can fail.
    pub fn try_for_each_line<X>(
        &self,
        emit: impl FnMut(&str) -> Result<(), X>,
    ) -> Result<(), ErLineError<X>> {
        lines::try_for_each_line(self, emit)
    }
}
impl Deref for ErSnapshotReport<'_> {
    type Target = ErSnapshot;

    fn deref(&self) -> &Self::Target {
        self.snapshot
    }
}
impl fmt::Display for ErSnapshotReport<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut previous_depth = None;
        for entry in &self.snapshot.entries {
            if !valid_depth(previous_depth, entry.depth) {
                return formatter.write_str("ER_INVALID_SNAPSHOT");
            }
            previous_depth = Some(entry.depth);
        }

        write_entries(formatter, self.snapshot.er_entries(), self.layout)
    }
}
impl fmt::Debug for ErSnapshotReport<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, formatter)
    }
}
impl Error for ErSnapshotReport<'_> {}

impl ErSnapshotTop<'_> {
    /// Just the saved outer error as text, keeps this layout.
    pub fn er_top_string(&self) -> String {
        self.to_string()
    }

    /// The whole saved report as text, keeps this layout.
    pub fn er_report_string(&self) -> String {
        ErSnapshotReport {
            snapshot: self.snapshot,
            layout: self.layout,
        }
        .to_string()
    }

    /// Pick how it gets printed.
    pub const fn layout(self, layout: Layout) -> Self {
        Self { layout, ..self }
    }

    /// Print on one line.
    pub const fn single_line(self) -> Self {
        self.layout(Layout::SingleLine)
    }

    /// Only formats once, no line endings.
    pub fn for_each_line(&self, emit: impl FnMut(&str)) -> fmt::Result {
        lines::for_each_line(self, emit)
    }

    /// Like `for_each_line()`, but the callback can fail.
    pub fn try_for_each_line<X>(
        &self,
        emit: impl FnMut(&str) -> Result<(), X>,
    ) -> Result<(), ErLineError<X>> {
        lines::try_for_each_line(self, emit)
    }
}
impl Deref for ErSnapshotTop<'_> {
    type Target = ErSnapshot;

    fn deref(&self) -> &Self::Target {
        self.snapshot
    }
}
impl fmt::Display for ErSnapshotTop<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(entry) = self.snapshot.entries.first() {
            write_top(formatter, &entry.message, self.layout)?;
        }

        Ok(())
    }
}
impl fmt::Debug for ErSnapshotTop<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, formatter)
    }
}
impl Error for ErSnapshotTop<'_> {}
