#[cfg(any(er_unsync, target_has_atomic = "ptr"))]
use crate::ErShared;
#[cfg(feature = "src_locations")]
use crate::ErSnapshotLocation;
use crate::impls::saved::write_saved_parts;
use crate::lines;
use crate::render::{
    format_into, format_report_string,
    report::{valid_depth, write_report_entries},
};
use crate::{
    ErBoxedInput, ErEntries, ErEntryKind, ErLineError, ErReportError, ErSavedList, ErSnapshot,
    ErSnapshotEntry, ErTree, InternalSnapshotReportRef, IntoErSnapshot,
};
#[cfg(target_has_atomic = "ptr")]
use alloc::sync::Arc;
use alloc::{boxed::Box, rc::Rc, string::String, vec::Vec};
use core::{fmt, mem, slice};

impl ErSnapshot {
    /// Copy the saved entries.
    pub fn er_snapshot(&self) -> Self {
        self.clone()
    }

    /// The saved report as text.
    #[must_use]
    pub fn er_report(&self) -> String {
        let capacity = match self.entries.as_slice() {
            [entry] if entry.depth == 0 && self.saved.is_empty() => {
                #[cfg(feature = "src_locations")]
                let location = entry
                    .src_location
                    .as_ref()
                    .map_or(0, |location| location.file.len() + 32);
                #[cfg(not(feature = "src_locations"))]
                let location = 0;

                entry.message.trim_end_matches(['\r', '\n']).len() + location
            }
            [] if self.saved.is_empty() => return String::new(),
            _ => return format_report_string(&self.internal_report()),
        };
        format_into(String::with_capacity(capacity), &self.internal_report())
    }

    /// Write the saved report straight to a buffer, just like `ErTree::er_write_report`.
    pub fn er_write_report(&self, writer: &mut (impl fmt::Write + ?Sized)) -> fmt::Result {
        writer.write_fmt(format_args!("{}", self.internal_report()))
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

    /// Stored sub errors, skips the root and native sources.
    pub fn er_descendants(&self) -> impl Iterator<Item = &ErSnapshotEntry> {
        self.er_entries()
            .filter(|entry| entry.kind == ErEntryKind::Node)
    }

    /// Print the whole saved report.
    pub(crate) const fn internal_report(&self) -> InternalSnapshotReportRef<'_> {
        InternalSnapshotReportRef {
            entries: self.entries.as_slice(),
            saved: self.saved.as_slice(),
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

        format_report_string(&InternalSnapshotReportRef {
            entries: &self.entries,
            saved: &self.saved,
            extra: &extra,
        })
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

        while let Some(entry) = walk.next() {
            scratch.clear();
            let text = format_into(mem::take(&mut scratch), entry.error);

            // Later entries can reuse the buffer even if this message filled it exactly.
            let message = if text.len() == text.capacity() && walk.size_hint().1 == Some(0) {
                text
            } else {
                scratch = text;
                String::from(scratch.as_str())
            };

            #[cfg(feature = "src_locations")]
            let src_location = entry.src_location.map(|location| ErSnapshotLocation {
                file: location.file().into(),
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

        Self {
            entries,
            saved: Vec::new(),
        }
    }
}

impl fmt::Display for InternalSnapshotReportRef<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut previous_depth = None;
        for entry in self.entries {
            if !valid_depth(previous_depth, entry.depth) {
                return formatter.write_str("ER_INVALID_SNAPSHOT");
            }
            previous_depth = Some(entry.depth);
        }

        write_report_entries::<false, _>(formatter, self.entries)?;

        write_saved_parts(formatter, self.saved, self.extra)?;

        Ok(())
    }
}
