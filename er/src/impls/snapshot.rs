#[cfg(feature = "src_locations")]
use crate::ErSnapshotLocation;
use crate::lines;
use crate::render::{
    format_string,
    report::{valid_depth, write_entries},
    write_top,
};
use crate::{
    ErAsError, ErEntries, ErEntryKind, ErLineError, ErOpaqueErrorExt, ErSnapshot, ErSnapshotEntry,
    ErSnapshotReport, ErSnapshotReportRef, ErSnapshotTop, ErSnapshotTopRef, IntoErSnapshot, Layout,
};
use alloc::{
    string::{String, ToString},
    vec::Vec,
};
use core::{fmt, ops::Deref, slice};

impl ErSnapshot {
    /// Copy the saved entries.
    pub fn er_snapshot(&self) -> Self {
        self.clone()
    }

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
    pub const fn er_report(&self) -> ErSnapshotReportRef<'_> {
        ErSnapshotReportRef {
            snapshot: self,
            layout: Layout::Multiline,
        }
    }

    /// Print just the saved top error.
    pub const fn er_top(&self) -> ErSnapshotTopRef<'_> {
        ErSnapshotTopRef {
            snapshot: self,
            layout: Layout::Multiline,
        }
    }

    /// Print the whole saved report, moves the snapshot.
    pub const fn into_er_report(self) -> ErSnapshotReport {
        ErSnapshotReport {
            snapshot: self,
            layout: Layout::Multiline,
        }
    }

    /// Print just the saved top, moves the snapshot.
    pub const fn into_er_top(self) -> ErSnapshotTop {
        ErSnapshotTop {
            snapshot: self,
            layout: Layout::Multiline,
        }
    }
}
impl IntoErSnapshot for ErSnapshot {
    type Top = ErSnapshotTop;
    type Report = ErSnapshotReport;

    fn into_er_snapshot(self) -> Self {
        self
    }

    fn into_er_top(self) -> ErSnapshotTop {
        self.into_er_top()
    }

    fn into_er_report(self) -> ErSnapshotReport {
        self.into_er_report()
    }
}
impl<'a> IntoErSnapshot for &'a ErSnapshot {
    type Top = ErSnapshotTopRef<'a>;
    type Report = ErSnapshotReportRef<'a>;

    fn into_er_snapshot(self) -> ErSnapshot {
        self.clone()
    }

    fn into_er_top(self) -> Self::Top {
        self.er_top()
    }

    fn into_er_report(self) -> Self::Report {
        self.er_report()
    }
}
impl<'a, P: ?Sized> IntoErSnapshot for &'a mut P
where
    &'a P: IntoErSnapshot,
{
    type Top = <&'a P as IntoErSnapshot>::Top;
    type Report = <&'a P as IntoErSnapshot>::Report;

    fn into_er_snapshot(self) -> ErSnapshot {
        <&'a P as IntoErSnapshot>::into_er_snapshot(self)
    }

    fn into_er_top(self) -> Self::Top {
        <&'a P as IntoErSnapshot>::into_er_top(self)
    }

    fn into_er_report(self) -> Self::Report {
        <&'a P as IntoErSnapshot>::into_er_report(self)
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

impl<'a> ErSnapshotReportRef<'a> {
    /// The same saved report, keeping its layout.
    pub const fn er_report(&self) -> Self {
        *self
    }

    /// Just the saved top, keeping this layout.
    pub const fn er_top(&self) -> ErSnapshotTopRef<'a> {
        ErSnapshotTopRef {
            snapshot: self.snapshot,
            layout: self.layout,
        }
    }

    /// Copy the saved entries, including the whole tree.
    pub fn er_snapshot(&self) -> ErSnapshot {
        self.snapshot.clone()
    }

    /// The saved errors, in tree order.
    pub fn er_entries(&self) -> slice::Iter<'a, ErSnapshotEntry> {
        self.snapshot.er_entries()
    }

    /// Call once per saved error.
    pub fn er_for_each_entry(&self, visit: impl FnMut(&'a ErSnapshotEntry)) {
        self.snapshot.er_for_each_entry(visit);
    }

    /// Stored sub errors, skips the root and native sources.
    pub fn er_descendants(&self) -> impl Iterator<Item = &'a ErSnapshotEntry> + use<'a> {
        self.snapshot.er_descendants()
    }

    /// The saved report as text, keeps this layout.
    pub fn er_report_string(&self) -> String {
        self.to_string()
    }

    /// Just the saved outer error as text, keeps this layout.
    pub fn er_top_string(&self) -> String {
        ErSnapshotTopRef {
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
impl Deref for ErSnapshotReportRef<'_> {
    type Target = ErSnapshot;

    fn deref(&self) -> &Self::Target {
        self.snapshot
    }
}
impl<'a> From<ErSnapshotTopRef<'a>> for ErSnapshotReportRef<'a> {
    fn from(top: ErSnapshotTopRef<'a>) -> Self {
        top.er_report()
    }
}
impl fmt::Display for ErSnapshotReportRef<'_> {
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
impl fmt::Debug for ErSnapshotReportRef<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, formatter)
    }
}
impl ErOpaqueErrorExt for ErSnapshotReportRef<'_> {
    type Output = ErAsError<Self>;

    fn opaque_err(self) -> Self::Output {
        ErAsError(self)
    }
}
impl<'a> ErOpaqueErrorExt for &ErSnapshotReportRef<'a> {
    type Output = ErAsError<ErSnapshotReportRef<'a>>;

    fn opaque_err(self) -> Self::Output {
        self.er_report().opaque_err()
    }
}
impl<'a> IntoErSnapshot for ErSnapshotReportRef<'a> {
    type Top = ErSnapshotTopRef<'a>;
    type Report = Self;

    fn into_er_snapshot(self) -> ErSnapshot {
        self.snapshot.clone()
    }

    fn into_er_top(self) -> Self::Top {
        self.er_top()
    }

    fn into_er_report(self) -> Self {
        self
    }
}
impl<'a> IntoErSnapshot for &ErSnapshotReportRef<'a> {
    type Top = ErSnapshotTopRef<'a>;
    type Report = ErSnapshotReportRef<'a>;

    fn into_er_snapshot(self) -> ErSnapshot {
        self.er_snapshot()
    }

    fn into_er_top(self) -> Self::Top {
        self.er_top()
    }

    fn into_er_report(self) -> Self::Report {
        *self
    }
}

impl<'a> ErSnapshotTopRef<'a> {
    /// The same saved top, keeping its layout.
    pub const fn er_top(&self) -> Self {
        *self
    }

    /// The whole saved report, keeping this layout.
    pub const fn er_report(&self) -> ErSnapshotReportRef<'a> {
        ErSnapshotReportRef {
            snapshot: self.snapshot,
            layout: self.layout,
        }
    }

    /// Copy the saved entries, including the whole tree.
    pub fn er_snapshot(&self) -> ErSnapshot {
        self.snapshot.clone()
    }

    /// The saved errors, in tree order.
    pub fn er_entries(&self) -> slice::Iter<'a, ErSnapshotEntry> {
        self.snapshot.er_entries()
    }

    /// Call once per saved error.
    pub fn er_for_each_entry(&self, visit: impl FnMut(&'a ErSnapshotEntry)) {
        self.snapshot.er_for_each_entry(visit);
    }

    /// Stored sub errors, skips the root and native sources.
    pub fn er_descendants(&self) -> impl Iterator<Item = &'a ErSnapshotEntry> + use<'a> {
        self.snapshot.er_descendants()
    }

    /// Just the saved outer error as text, keeps this layout.
    pub fn er_top_string(&self) -> String {
        self.to_string()
    }

    /// The whole saved report as text, keeps this layout.
    pub fn er_report_string(&self) -> String {
        ErSnapshotReportRef {
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
impl Deref for ErSnapshotTopRef<'_> {
    type Target = ErSnapshot;

    fn deref(&self) -> &Self::Target {
        self.snapshot
    }
}
impl<'a> From<ErSnapshotReportRef<'a>> for ErSnapshotTopRef<'a> {
    fn from(report: ErSnapshotReportRef<'a>) -> Self {
        report.er_top()
    }
}
impl fmt::Display for ErSnapshotTopRef<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(entry) = self.snapshot.entries.first() {
            write_top(formatter, &entry.message, self.layout)?;
        }

        Ok(())
    }
}
impl fmt::Debug for ErSnapshotTopRef<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, formatter)
    }
}
impl ErOpaqueErrorExt for ErSnapshotTopRef<'_> {
    type Output = ErAsError<Self>;

    fn opaque_err(self) -> Self::Output {
        ErAsError(self)
    }
}
impl<'a> ErOpaqueErrorExt for &ErSnapshotTopRef<'a> {
    type Output = ErAsError<ErSnapshotTopRef<'a>>;

    fn opaque_err(self) -> Self::Output {
        self.er_top().opaque_err()
    }
}
impl<'a> IntoErSnapshot for ErSnapshotTopRef<'a> {
    type Top = Self;
    type Report = ErSnapshotReportRef<'a>;

    fn into_er_snapshot(self) -> ErSnapshot {
        self.snapshot.clone()
    }

    fn into_er_top(self) -> Self {
        self
    }

    fn into_er_report(self) -> Self::Report {
        self.er_report()
    }
}
impl<'a> IntoErSnapshot for &ErSnapshotTopRef<'a> {
    type Top = ErSnapshotTopRef<'a>;
    type Report = ErSnapshotReportRef<'a>;

    fn into_er_snapshot(self) -> ErSnapshot {
        self.er_snapshot()
    }

    fn into_er_top(self) -> Self::Top {
        *self
    }

    fn into_er_report(self) -> Self::Report {
        self.er_report()
    }
}

impl ErSnapshotReport {
    /// Copy the saved entries, including the whole tree.
    pub fn er_snapshot(&self) -> ErSnapshot {
        self.snapshot.clone()
    }

    /// Borrow the same report, keeping its layout.
    pub const fn as_ref(&self) -> ErSnapshotReportRef<'_> {
        ErSnapshotReportRef {
            snapshot: &self.snapshot,
            layout: self.layout,
        }
    }

    /// The same saved report, keeping its layout.
    pub const fn er_report(&self) -> ErSnapshotReportRef<'_> {
        self.as_ref()
    }

    /// Just the saved top, keeping this layout.
    pub const fn er_top(&self) -> ErSnapshotTopRef<'_> {
        ErSnapshotTopRef {
            snapshot: &self.snapshot,
            layout: self.layout,
        }
    }

    /// The saved report as text, keeps this layout.
    pub fn er_report_string(&self) -> String {
        self.er_report().er_report_string()
    }

    /// Just the saved outer error as text, keeps this layout.
    pub fn er_top_string(&self) -> String {
        self.er_top().er_top_string()
    }

    /// Pick how it gets printed.
    pub fn layout(self, layout: Layout) -> Self {
        Self { layout, ..self }
    }

    /// Print on one line.
    pub fn single_line(self) -> Self {
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
impl Deref for ErSnapshotReport {
    type Target = ErSnapshot;

    fn deref(&self) -> &Self::Target {
        &self.snapshot
    }
}
impl AsMut<ErSnapshot> for ErSnapshotReport {
    fn as_mut(&mut self) -> &mut ErSnapshot {
        &mut self.snapshot
    }
}
impl fmt::Display for ErSnapshotReport {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.er_report(), formatter)
    }
}
impl fmt::Debug for ErSnapshotReport {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, formatter)
    }
}
impl ErOpaqueErrorExt for ErSnapshotReport {
    type Output = ErAsError<Self>;

    fn opaque_err(self) -> Self::Output {
        ErAsError(self)
    }
}
impl<'a> ErOpaqueErrorExt for &'a ErSnapshotReport {
    type Output = ErAsError<ErSnapshotReportRef<'a>>;

    fn opaque_err(self) -> Self::Output {
        self.er_report().opaque_err()
    }
}
impl IntoErSnapshot for ErSnapshotReport {
    type Top = ErSnapshotTop;
    type Report = Self;

    fn into_er_snapshot(self) -> ErSnapshot {
        self.snapshot
    }

    fn into_er_top(self) -> ErSnapshotTop {
        ErSnapshotTop {
            snapshot: self.snapshot,
            layout: self.layout,
        }
    }

    fn into_er_report(self) -> Self {
        self
    }
}
impl<'a> IntoErSnapshot for &'a ErSnapshotReport {
    type Top = ErSnapshotTopRef<'a>;
    type Report = ErSnapshotReportRef<'a>;

    fn into_er_snapshot(self) -> ErSnapshot {
        self.er_snapshot()
    }

    fn into_er_top(self) -> Self::Top {
        self.er_top()
    }

    fn into_er_report(self) -> Self::Report {
        self.er_report()
    }
}
impl From<ErSnapshot> for ErSnapshotReport {
    fn from(snapshot: ErSnapshot) -> Self {
        snapshot.into_er_report()
    }
}
impl From<ErSnapshotTop> for ErSnapshotReport {
    fn from(top: ErSnapshotTop) -> Self {
        top.into_er_report()
    }
}
impl From<ErSnapshotReport> for ErSnapshot {
    fn from(report: ErSnapshotReport) -> Self {
        report.snapshot
    }
}

impl ErSnapshotTop {
    /// Copy the saved entries, including the whole tree.
    pub fn er_snapshot(&self) -> ErSnapshot {
        self.snapshot.clone()
    }

    /// Borrow the same top, keeping its layout.
    pub const fn as_ref(&self) -> ErSnapshotTopRef<'_> {
        ErSnapshotTopRef {
            snapshot: &self.snapshot,
            layout: self.layout,
        }
    }

    /// The same saved top, keeping its layout.
    pub const fn er_top(&self) -> ErSnapshotTopRef<'_> {
        self.as_ref()
    }

    /// The whole saved report, keeping this layout.
    pub const fn er_report(&self) -> ErSnapshotReportRef<'_> {
        ErSnapshotReportRef {
            snapshot: &self.snapshot,
            layout: self.layout,
        }
    }

    /// Just the saved outer error as text, keeps this layout.
    pub fn er_top_string(&self) -> String {
        self.er_top().er_top_string()
    }

    /// The whole saved report as text, keeps this layout.
    pub fn er_report_string(&self) -> String {
        self.er_report().er_report_string()
    }

    /// Pick how it gets printed.
    pub fn layout(self, layout: Layout) -> Self {
        Self { layout, ..self }
    }

    /// Print on one line.
    pub fn single_line(self) -> Self {
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
impl Deref for ErSnapshotTop {
    type Target = ErSnapshot;

    fn deref(&self) -> &Self::Target {
        &self.snapshot
    }
}
impl AsMut<ErSnapshot> for ErSnapshotTop {
    fn as_mut(&mut self) -> &mut ErSnapshot {
        &mut self.snapshot
    }
}
impl fmt::Display for ErSnapshotTop {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.er_top(), formatter)
    }
}
impl fmt::Debug for ErSnapshotTop {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, formatter)
    }
}
impl ErOpaqueErrorExt for ErSnapshotTop {
    type Output = ErAsError<Self>;

    fn opaque_err(self) -> Self::Output {
        ErAsError(self)
    }
}
impl<'a> ErOpaqueErrorExt for &'a ErSnapshotTop {
    type Output = ErAsError<ErSnapshotTopRef<'a>>;

    fn opaque_err(self) -> Self::Output {
        self.er_top().opaque_err()
    }
}
impl IntoErSnapshot for ErSnapshotTop {
    type Top = Self;
    type Report = ErSnapshotReport;

    fn into_er_snapshot(self) -> ErSnapshot {
        self.snapshot
    }

    fn into_er_top(self) -> Self {
        self
    }

    fn into_er_report(self) -> ErSnapshotReport {
        ErSnapshotReport {
            snapshot: self.snapshot,
            layout: self.layout,
        }
    }
}
impl<'a> IntoErSnapshot for &'a ErSnapshotTop {
    type Top = ErSnapshotTopRef<'a>;
    type Report = ErSnapshotReportRef<'a>;

    fn into_er_snapshot(self) -> ErSnapshot {
        self.er_snapshot()
    }

    fn into_er_top(self) -> Self::Top {
        self.er_top()
    }

    fn into_er_report(self) -> Self::Report {
        self.er_report()
    }
}
impl From<ErSnapshot> for ErSnapshotTop {
    fn from(snapshot: ErSnapshot) -> Self {
        snapshot.into_er_top()
    }
}
impl From<ErSnapshotReport> for ErSnapshotTop {
    fn from(report: ErSnapshotReport) -> Self {
        report.into_er_top()
    }
}
impl From<ErSnapshotTop> for ErSnapshot {
    fn from(top: ErSnapshotTop) -> Self {
        top.snapshot
    }
}
