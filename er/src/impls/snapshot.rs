#[cfg(any(er_unsync, target_has_atomic = "ptr"))]
use crate::ErShared;
use crate::ErSnapshotLocation;
use crate::impls::saved::{SnapshotSaves, write_saved_parts};
use crate::lines;
use crate::render::{
    REPORT_CAPACITY,
    report::{valid_depth, write_entries_with_saved},
    write_args,
};
use crate::{
    ErBoxedInput, ErEntries, ErEntryKind, ErLineError, ErNode, ErReportError, ErSaved, ErSavedList,
    ErSnapshot, ErSnapshotEntry, ErSrcLocation, ErTree, InternalSnapshotReportRef, IntoErSnapshot,
};
#[cfg(target_has_atomic = "ptr")]
use alloc::sync::Arc;
use alloc::{boxed::Box, rc::Rc, string::String, vec::Vec};
use core::{error::Error, fmt, mem, slice};

impl ErSnapshot {
    pub(crate) fn capture(
        error: &(dyn Error + 'static),
        nodes: &[ErNode],
        saved: &ErSavedList,
        src_location: Option<ErSrcLocation>,
    ) -> Self {
        ErEntries::with_saved(error, nodes, src_location, saved).into()
    }

    /// Copy the saved entries.
    pub fn er_snapshot(&self) -> Self {
        self.clone()
    }

    /// The saved report as text.
    #[must_use]
    pub fn er_report(&self) -> String {
        self.internal_report().render_string()
    }

    /// Write the saved report straight to a buffer, just like `ErTree::er_write_report`.
    pub fn er_write_report(&self, mut writer: &mut (impl fmt::Write + ?Sized)) -> fmt::Result {
        self.internal_report().write_to(&mut writer)
    }

    /// The saved top error's message.
    pub fn top(&self) -> &str {
        self.entries
            .first()
            .map_or("", |entry| entry.message.as_str())
    }

    /// The saved report a line at a time, without line endings. Only formats once.
    pub fn for_each_line(&self, emit: impl FnMut(&str)) -> fmt::Result {
        lines::for_each_line(&self.internal_report(), emit)
    }

    /// Like `for_each_line()`, but the callback can fail.
    pub fn try_for_each_line<X>(
        &self,
        emit: impl FnMut(&str) -> Result<(), X>,
    ) -> Result<(), ErLineError<X>> {
        lines::try_for_each_line(&self.internal_report(), emit)
    }

    /// The saved errors, in tree order.
    pub fn er_entries(&self) -> slice::Iter<'_, ErSnapshotEntry> {
        self.entries.iter()
    }

    /// Reports on each row in tree order, followed by any without a row.
    pub fn er_saved(&self) -> impl Iterator<Item = &ErSaved> {
        self.entries
            .iter()
            .flat_map(|entry| &entry.saved)
            .chain(&self.unattached)
    }

    /// Stored sub errors, skips the root and native sources.
    pub fn er_descendants(&self) -> impl Iterator<Item = &ErSnapshotEntry> {
        self.er_entries()
            .filter(|entry| entry.kind == ErEntryKind::Node)
    }

    /// Print the whole saved report.
    pub(crate) fn internal_report(&self) -> InternalSnapshotReportRef<'_> {
        InternalSnapshotReportRef {
            entries: self.entries.as_slice(),
            unattached: &self.unattached,
            extra: &[],
        }
    }
}
impl IntoErSnapshot for ErSnapshot {
    fn into_er_snapshot(self) -> Self {
        self
    }

    fn into_er_report_text(self) -> String {
        self.er_report()
    }

    fn into_er_report_with_saved(self, extra: ErSavedList) -> String {
        (&self).into_er_report_with_saved(extra)
    }
}
impl IntoErSnapshot for &ErSnapshot {
    fn into_er_snapshot(self) -> ErSnapshot {
        self.clone()
    }

    fn into_er_report_text(self) -> String {
        self.er_report()
    }

    fn into_er_report_with_saved(self, extra: ErSavedList) -> String {
        if extra.is_empty() {
            return self.er_report();
        }

        InternalSnapshotReportRef {
            extra: &extra,
            ..self.internal_report()
        }
        .render_string()
    }
}
// A wrapper around something that prints as a snapshot hands every method on. They're written once here,
// the wrappers kept forgetting a method each time the trait got a new one.
macro_rules! forward_snapshot {
    ([$($generics:tt)*] $outer:ty, [$($inner:tt)+], |$this:ident| $get:expr) => {
        impl<$($generics)* Input> IntoErSnapshot<Input> for $outer
        where
            $($inner)+: IntoErSnapshot<Input>,
        {
            fn into_er_snapshot(self) -> ErSnapshot {
                let $this = self;
                IntoErSnapshot::<Input>::into_er_snapshot($get)
            }

            fn into_er_report_text(self) -> String {
                let $this = self;
                IntoErSnapshot::<Input>::into_er_report_text($get)
            }

            fn into_er_report_with_saved(self, saved: ErSavedList) -> String {
                let $this = self;
                IntoErSnapshot::<Input>::into_er_report_with_saved($get, saved)
            }
        }
    };
}
forward_snapshot!([P,] Box<P>, [P], |this| *this);
forward_snapshot!(['a, P: ?Sized,] &'a Box<P>, [&'a P], |this| &**this);
forward_snapshot!(['a, P: ?Sized,] &&'a P, [&'a P], |this| *this);
forward_snapshot!(['a, P: ?Sized,] &'a &mut P, [&'a P], |this| &**this);
forward_snapshot!(['a, P: ?Sized,] &'a mut P, [&'a P], |this| &*this);

macro_rules! shared_presentation {
    ($pointer:ident, $($input:ty),+) => {
        forward_snapshot!(['a, P: ?Sized,] &'a $pointer<P>, [&'a P], |this| &**this);
        forward_snapshot!([P: ?Sized,] $pointer<P>, [for<'a> &'a P], |this| &*this);

        $(
            impl<E: ErReportError<$input>> IntoErSnapshot<$input> for $pointer<ErTree<E>> {
                fn into_er_snapshot(self) -> ErSnapshot {
                    self.er_snapshot()
                }

                fn into_er_report_text(self) -> String {
                    self.er_report()
                }
            }
        )+
    };
}

shared_presentation!(Rc, (), ErBoxedInput);
#[cfg(target_has_atomic = "ptr")]
shared_presentation!(Arc, (), ErBoxedInput);
#[cfg(any(er_unsync, target_has_atomic = "ptr"))]
shared_presentation!(ErShared, (), ErBoxedInput);

impl From<ErEntries<'_>> for ErSnapshot {
    fn from(mut walk: ErEntries<'_>) -> Self {
        let mut entries = Vec::with_capacity(walk.size_hint().0);
        let mut scratch = String::with_capacity(128);
        let mut saves = SnapshotSaves::default();

        while let Some(entry) = walk.next() {
            scratch.clear();
            write_args(&mut scratch, format_args!("{}", entry.error));

            // Later entries can reuse the buffer even if this message filled it exactly.
            let message = if scratch.len() == scratch.capacity() && walk.size_hint().1 == Some(0) {
                mem::take(&mut scratch)
            } else {
                String::from(scratch.as_str())
            };

            let src_location = entry.src_location.map(|location| ErSnapshotLocation {
                file: location.file().into(),
                line: location.line(),
                column: location.column(),
            });

            entries.push(ErSnapshotEntry {
                message,
                saved: saves.row(entry.shared_saved, entry.saved),
                kind: entry.kind,
                index: entry.index,
                parent: entry.parent,
                depth: entry.depth,
                is_last: entry.is_last,
                source_truncated: entry.source_truncated,
                src_location,
            });
        }

        Self {
            entries,
            unattached: ErSavedList::default(),
        }
    }
}

impl InternalSnapshotReportRef<'_> {
    fn render_string(&self) -> String {
        let mut capacity = 0usize;
        let valid = self.check_entries(|entry| {
            let message = entry.message.trim_end_matches(['\r', '\n']);
            capacity = capacity
                .saturating_add(message.len() + entry.depth * 3 + usize::from(entry.depth != 0));
        });
        if !valid {
            return String::from("ER_INVALID_SNAPSHOT");
        }

        if let Some(location) = self
            .entries
            .first()
            .and_then(|entry| entry.src_location.as_ref())
        {
            capacity = capacity.saturating_add(location.file.len() + 32);
        }
        if self.entries.len() > 1
            || !self.extra.is_empty()
            || !self.unattached.is_empty()
            || self
                .entries
                .first()
                .is_some_and(|root| !root.saved.is_empty())
        {
            capacity = capacity.max(REPORT_CAPACITY);
        }

        let mut report = String::with_capacity(capacity);
        if self.write_rows(&mut report).is_err() {
            report.clear();
            report.push_str("ER_FMT_FAILED");
        }
        report
    }

    fn write_to(&self, writer: &mut dyn fmt::Write) -> fmt::Result {
        if self.check_entries(|_| {}) {
            self.write_rows(writer)
        } else {
            writer.write_str("ER_INVALID_SNAPSHOT")
        }
    }

    fn check_entries(&self, mut visit: impl FnMut(&ErSnapshotEntry)) -> bool {
        let mut previous_depth = None;
        for entry in self.entries {
            if !valid_depth(previous_depth, entry.depth) {
                return false;
            }

            previous_depth = Some(entry.depth);
            visit(entry);
        }
        true
    }

    fn write_rows(&self, writer: &mut dyn fmt::Write) -> fmt::Result {
        if self.entries.is_empty() {
            return write_saved_parts(writer, self.unattached, self.extra);
        }

        write_entries_with_saved(writer, self.entries.iter().enumerate(), |(index, entry)| {
            let extra = if index == 0 { self.extra } else { &[] };
            let has_saves = !entry.saved.is_empty() || !extra.is_empty();

            (
                entry.report_entry(),
                has_saves.then_some((&entry.saved, extra)),
            )
        })?;

        write_saved_parts(writer, self.unattached, &[])
    }
}
impl fmt::Display for InternalSnapshotReportRef<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.write_to(formatter)
    }
}
