use crate::lines;
use crate::render::{report::write_entries, write_top};
use crate::{ErSnapshot, ErSnapshotReport, ErSnapshotTop, Layout, LineError};
use core::{error::Error, fmt, ops::Deref};

impl ErSnapshotReport<'_> {
    pub const fn layout(self, layout: Layout) -> Self {
        Self { layout, ..self }
    }

    pub const fn single_line(self) -> Self {
        self.layout(Layout::SingleLine)
    }

    /// Only formats once, no line endings.
    pub fn for_each_line(&self, emit: impl FnMut(&str)) -> fmt::Result {
        lines::for_each_line(self, emit)
    }

    pub fn try_for_each_line<X>(
        &self,
        emit: impl FnMut(&str) -> Result<(), X>,
    ) -> Result<(), LineError<X>> {
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
    pub const fn layout(self, layout: Layout) -> Self {
        Self { layout, ..self }
    }

    pub const fn single_line(self) -> Self {
        self.layout(Layout::SingleLine)
    }

    /// Only formats once, no line endings.
    pub fn for_each_line(&self, emit: impl FnMut(&str)) -> fmt::Result {
        lines::for_each_line(self, emit)
    }

    pub fn try_for_each_line<X>(
        &self,
        emit: impl FnMut(&str) -> Result<(), X>,
    ) -> Result<(), LineError<X>> {
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
