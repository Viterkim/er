use crate::impls::error::report_string;
use crate::render::LineWriter;
use crate::types::Shared;
use crate::{
    BoxError, ErBuildFrom, ErEntries, ErErrorContextExt, ErErrorExt, ErErrorWithSaved, ErPart,
    ErPresentationExt, ErReady, ErReportText, ErResult, ErResultSaveExt, ErSaveExt, ErSaveInput,
    ErSaveRef, ErSaved, ErSavedKind, ErSavedList, ErSavedSnapshot, ErSavedTree, ErSnapshot, ErTree,
    ErWithReport, ErWithSaves, IntoErPart, IntoErSnapshot, IntoErTree,
};
use crate::{ErBoxedInput, ErReportError, ErWithFields};
#[cfg(any(er_unsync, target_has_atomic = "ptr"))]
use crate::{ErShareReportExt, ErShared, ErSharedTree};
#[cfg(target_has_atomic = "ptr")]
use alloc::sync::Arc;
#[cfg(feature = "stack_traces")]
use alloc::{borrow::Cow, string::ToString};
use alloc::{borrow::ToOwned, boxed::Box, rc::Rc, string::String, vec::Vec};
use core::{
    any::type_name,
    borrow::Borrow,
    error::Error,
    fmt::{self, Write as _},
    hash::{Hash, Hasher},
    ops::Deref,
    slice,
};
#[cfg(feature = "stack_traces")]
use std::{
    backtrace::{Backtrace, BacktraceStatus},
    collections::{HashMap, HashSet},
};

impl ErSavedList {
    pub fn push(&mut self, saved: ErSaved) {
        match self.0.as_mut() {
            Some(entries) => saved_entries_mut(entries).push(saved),
            None => self.0 = Some(Shared::new(alloc::vec![saved])),
        }
    }

    /// Move all of `other` to the end of this one. A list shared with this one is already here and stays as it is.
    #[inline]
    pub fn append(&mut self, other: &mut Self) {
        if let Some(incoming) = other.0.take() {
            self.append_some(incoming);
        }
    }

    #[cold]
    #[inline(never)]
    fn append_some(&mut self, incoming: Shared<Vec<ErSaved>>) {
        let Some(entries) = self.0.as_mut() else {
            self.0 = Some(incoming);
            return;
        };

        // The same list again is one shared failure sitting under two parents, its reports are already here.
        #[cfg(any(er_unsync, target_has_atomic = "ptr"))]
        if Shared::ptr_eq(entries, &incoming) {
            return;
        }

        let entries = saved_entries_mut(entries);
        #[cfg(any(er_unsync, target_has_atomic = "ptr"))]
        match Shared::try_unwrap(incoming) {
            Ok(mut owned) => entries.append(&mut owned),
            Err(shared) => extend_missing(entries, &shared),
        }
        #[cfg(not(any(er_unsync, target_has_atomic = "ptr")))]
        {
            let mut incoming = incoming;
            entries.append(&mut incoming);
        }
    }
}

/// Copy over the reports this list doesn't have yet. The same shared failure reaching one parent through several
/// paths, `[&shared, &shared]`, would copy every report again per path, and double the list for every layer
/// that does it. The report prints a duplicate once anyway.
///
/// A repeat lands right after the copy before it, give or take a note, so only the end of the list gets checked.
/// Checking all of it made an aggregate of 16,000 different shared trees take 33 seconds.
#[cfg(any(er_unsync, target_has_atomic = "ptr"))]
fn extend_missing(entries: &mut Vec<ErSaved>, incoming: &[ErSaved]) {
    let start = entries.len().saturating_sub(incoming.len() + 4);
    let end = entries.len();

    if incoming.len() * (end - start) <= 4096 {
        for entry in incoming {
            if !entries[start..end].contains(entry) {
                entries.push(entry.clone());
            }
        }

        return;
    }

    let mut index: Vec<(u64, usize)> = (start..end)
        .map(|position| (saved_hash(&entries[position]), position))
        .collect();
    index.sort_unstable();

    for entry in incoming {
        let hash = saved_hash(entry);
        let first = index.partition_point(|&(other, _)| other < hash);
        let present = index[first..]
            .iter()
            .take_while(|&&(other, _)| other == hash)
            .any(|&(_, position)| entries[position] == *entry);

        if !present {
            entries.push(entry.clone());
        }
    }
}

fn saved_hash(entry: &ErSaved) -> u64 {
    let mut hasher = Fnv(0xcbf2_9ce4_8422_2325);
    entry.hash(&mut hasher);

    hasher.0
}

fn saved_entries_mut(entries: &mut Shared<Vec<ErSaved>>) -> &mut Vec<ErSaved> {
    #[cfg(any(er_unsync, target_has_atomic = "ptr"))]
    {
        Shared::make_mut(entries)
    }
    #[cfg(not(any(er_unsync, target_has_atomic = "ptr")))]
    {
        entries
    }
}
impl Deref for ErSavedList {
    type Target = [ErSaved];

    fn deref(&self) -> &[ErSaved] {
        self.0.as_deref().map_or(&[], Vec::as_slice)
    }
}
impl PartialEq for ErSavedList {
    fn eq(&self, other: &Self) -> bool {
        self[..] == other[..]
    }
}
impl Eq for ErSavedList {}

impl<'a> IntoIterator for &'a ErSavedList {
    type Item = &'a ErSaved;
    type IntoIter = slice::Iter<'a, ErSaved>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl fmt::Display for ErSaved {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(formatter, "---- Saved: {} ----", self.title)?;
        let mut body = LineWriter::indent(formatter, "");
        write!(body, "{}", self.kind)
    }
}
impl fmt::Display for ErSavedKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Text(text) => formatter.write_str(text),
            Self::Snapshot(snapshot) => fmt::Display::fmt(&snapshot.internal_report(), formatter),
            #[cfg(feature = "stack_traces")]
            Self::StackTrace(trace) => fmt::Display::fmt(trace, formatter),
        }
    }
}
impl PartialEq for ErSavedKind {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Text(this), Self::Text(other)) => this == other,
            (Self::Snapshot(this), Self::Snapshot(other)) => this == other,
            #[cfg(feature = "stack_traces")]
            (Self::StackTrace(this), Self::StackTrace(other)) => Arc::ptr_eq(this, other),
            _ => false,
        }
    }
}
impl Eq for ErSavedKind {}
impl Hash for ErSavedKind {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            Self::Text(text) => {
                state.write_u8(0);
                text.hash(state);
            }
            Self::Snapshot(snapshot) => {
                state.write_u8(1);
                snapshot.hash(state);
            }
            // Two traces are the same when they're the same capture, like `eq` above.
            #[cfg(feature = "stack_traces")]
            Self::StackTrace(trace) => {
                state.write_u8(2);
                Arc::as_ptr(trace).hash(state);
            }
        }
    }
}

impl From<String> for ErSavedKind {
    fn from(text: String) -> Self {
        Self::Text(text)
    }
}
impl From<&str> for ErSavedKind {
    fn from(text: &str) -> Self {
        Self::Text(text.to_owned())
    }
}
impl From<&String> for ErSavedKind {
    fn from(text: &String) -> Self {
        Self::Text(text.clone())
    }
}
impl From<ErSnapshot> for ErSavedKind {
    fn from(snapshot: ErSnapshot) -> Self {
        Self::Snapshot(snapshot)
    }
}
impl From<&ErSnapshot> for ErSavedKind {
    fn from(snapshot: &ErSnapshot) -> Self {
        Self::Snapshot(snapshot.clone())
    }
}
#[cfg(feature = "stack_traces")]
impl From<Backtrace> for ErSavedKind {
    fn from(trace: Backtrace) -> Self {
        Self::StackTrace(Arc::new(trace))
    }
}

macro_rules! save_one {
    ($($ty:ty),*) => {
        $(
            impl ErSaveInput for $ty {
                fn er_save_into(self, title: String, saved: &mut ErSavedList) {
                    saved.push(ErSaved {
                        title,
                        kind: self.into(),
                    });
                }
            }
        )*
    };
}
save_one!(String, ErSnapshot);

macro_rules! save_reference {
    ($($ty:ty),*) => {
        save_one!($(&$ty),*);

        $(
            impl ErSaveInput for &&$ty {
                fn er_save_into(self, title: String, saved: &mut ErSavedList) {
                    (*self).er_save_into(title, saved);
                }
            }
        )*
    };
}
save_reference!(str, String, ErSnapshot);
#[cfg(feature = "stack_traces")]
save_one!(Backtrace);

impl<T: ErSaveInput> ErSaveInput for Option<T> {
    fn er_save_into(self, title: String, saved: &mut ErSavedList) {
        if let Some(item) = self {
            item.er_save_into(title, saved);
        }
    }
}
impl<'a, T> ErSaveInput for &'a Option<T>
where
    &'a T: ErSaveInput,
{
    fn er_save_into(self, title: String, saved: &mut ErSavedList) {
        if let Some(item) = self {
            item.er_save_into(title, saved);
        }
    }
}
impl ErSaveRef for str {
    fn er_save_ref(&self, title: String, saved: &mut ErSavedList) {
        self.er_save_into(title, saved);
    }
}
impl ErSaveRef for String {
    fn er_save_ref(&self, title: String, saved: &mut ErSavedList) {
        self.as_str().er_save_into(title, saved);
    }
}
impl ErSaveRef for ErSnapshot {
    fn er_save_ref(&self, title: String, saved: &mut ErSavedList) {
        self.er_save_into(title, saved);
    }
}
impl<T: ErSaveRef> ErSaveRef for Option<T> {
    fn er_save_ref(&self, title: String, saved: &mut ErSavedList) {
        if let Some(item) = self {
            item.er_save_ref(title, saved);
        }
    }
}
impl<T: ErSaveRef + ?Sized> ErSaveRef for &T {
    fn er_save_ref(&self, title: String, saved: &mut ErSavedList) {
        (*self).er_save_ref(title, saved);
    }
}

#[diagnostic::do_not_recommend]
impl<F: FnOnce() -> T, T: ErSaveInput> ErSaveInput for F {
    fn er_save_into(self, title: String, saved: &mut ErSavedList) {
        self().er_save_into(title, saved);
    }
}

impl<T> ErErrorWithSaved<T> {
    #[doc(hidden)]
    pub const fn new(value: T) -> Self {
        Self {
            value,
            saved: ErSavedList(None),
        }
    }

    fn saving(mut self, title: impl Into<String>, saved: impl ErSaveInput) -> Self {
        saved.er_save_into(title.into(), &mut self.saved);
        self
    }

    #[doc(hidden)]
    #[track_caller]
    pub fn build<A: ErBuildFrom<T, Form>, Form>(self) -> ErErrorWithSaved<A> {
        ErErrorWithSaved {
            value: A::er_build_from(self.value),
            saved: self.saved,
        }
    }
}

impl<T, E> ErResultSaveExt for Result<T, E> {
    type Ok = T;
    type Err = E;

    fn er_save<S: ErSaveRef + ?Sized>(
        self,
        title: impl Into<String>,
        saved: impl FnOnce(&E) -> &S,
    ) -> Result<T, ErErrorWithSaved<E>> {
        match self {
            Ok(value) => Ok(value),
            Err(error) => {
                let mut result = ErErrorWithSaved::new(error);
                saved(&result.value).er_save_ref(title.into(), &mut result.saved);
                Err(result)
            }
        }
    }
}

impl<E> ErErrorWithSaved<E> {
    /// Share this error with its saved reports.
    #[cfg(any(er_unsync, target_has_atomic = "ptr"))]
    #[track_caller]
    pub fn er_share<Input>(self) -> ErSharedTree<E::Error>
    where
        E: ErSavedError<Input>,
    {
        self.into_tree().er_share()
    }

    /// The tree with your error on top and the reports saved, when you need it in hand. `er_bail!` does this for you.
    #[track_caller]
    pub fn into_tree<Input>(self) -> ErTree<E::Error>
    where
        E: ErSavedError<Input>,
    {
        self.value.into_saved_tree(self.saved)
    }
}

#[doc(hidden)]
pub trait ErSavedError<Input>: Sized {
    type Error;

    #[track_caller]
    fn into_saved_tree(self, saved: ErSavedList) -> ErTree<Self::Error>;
}
macro_rules! save_plain_error {
    ($kind:ty, $presentation:ty) => {
        impl<E: ErReportError<$kind>> ErSavedError<$kind> for E {
            type Error = E;

            #[track_caller]
            fn into_saved_tree(self, saved: ErSavedList) -> ErTree<E> {
                let mut tree = ErTree::from_top(self);
                tree.saved = saved;
                tree
            }
        }

        impl<T, E: ErSavedError<$kind>> ErPresentationExt<$presentation>
            for Result<T, ErErrorWithSaved<E>>
        {
            type Ok = T;
            type Err = E::Error;

            #[track_caller]
            fn er_tree(self) -> ErResult<T, E::Error> {
                match self {
                    Ok(value) => Ok(value),
                    Err(ErErrorWithSaved { value, saved }) => Err(value.into_saved_tree(saved)),
                }
            }
        }
    };
}
save_plain_error!((), ErWithSaves);
save_plain_error!(ErBoxedInput, (ErWithSaves, ErBoxedInput));
impl<E: ErReportError<Input>, R: ErReportText, Input> ErSavedError<ErWithReport<Input>> for (E, R) {
    type Error = E;

    #[track_caller]
    fn into_saved_tree(self, saved: ErSavedList) -> ErTree<E> {
        report_pair_tree((self.0, self.1.into_er_report_string()), saved)
    }
}
impl<E> ErSavedError<ErSavedTree> for ErTree<E> {
    type Error = E;

    fn into_saved_tree(mut self, mut saved: ErSavedList) -> Self {
        self.saved.append(&mut saved);
        self
    }
}
impl ErSavedError<ErSavedTree> for crate::ErFailure {
    type Error = crate::ErFailed;

    fn into_saved_tree(self, saved: ErSavedList) -> ErTree<Self::Error> {
        self.into_er_tree().into_saved_tree(saved)
    }
}
impl<E: ErSavedError<ErSavedTree>> ErSavedError<ErSavedTree> for Box<E> {
    type Error = E::Error;

    fn into_saved_tree(self, saved: ErSavedList) -> ErTree<Self::Error> {
        (*self).into_saved_tree(saved)
    }
}
impl<E: ErSavedError<ErSavedTree>> IntoErTree for ErErrorWithSaved<E> {
    type Error = E::Error;

    fn into_er_tree(self) -> ErTree<Self::Error> {
        self.into_tree()
    }
}
impl<E: ErSavedError<Kind>, Kind> ErSavedError<Kind> for ErErrorWithSaved<E> {
    type Error = E::Error;

    fn into_saved_tree(self, mut saved: ErSavedList) -> ErTree<Self::Error> {
        let Self {
            value,
            saved: mut previous,
        } = self;
        previous.append(&mut saved);
        value.into_saved_tree(previous)
    }
}
impl<W: IntoErTree> From<ErErrorWithSaved<W>> for ErTree<W::Error> {
    fn from(error: ErErrorWithSaved<W>) -> Self {
        let ErErrorWithSaved { value, saved } = error;
        value.into_er_tree().into_saved_tree(saved)
    }
}
impl ErReportText for String {
    fn er_report_str(&self) -> Option<&str> {
        Some(self)
    }

    fn into_er_report_string(self) -> Option<String> {
        Some(self)
    }
}
impl ErReportText for Option<String> {
    fn er_report_str(&self) -> Option<&str> {
        self.as_deref()
    }

    fn into_er_report_string(self) -> Option<String> {
        self
    }
}

/// Panics with the report as it is, a String's own Debug would quote it.
pub(crate) struct ReportText(pub String);
impl fmt::Debug for ReportText {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// A library's error with its report next to it, like `Result<T, (StartError, String)>`.
/// `.er()` puts the error in the tree and saves the report under the error's type, like `aktor::setup::AktorStartError`.
/// An `Option<String>` report that's `None` just saves nothing.
impl<E: Error + Into<BoxError> + 'static, R: ErReportText> IntoErPart<ErWithReport> for (E, R) {
    type Error = E;

    fn er_error(&self) -> &E {
        &self.0
    }

    #[track_caller]
    fn into_er_part(self) -> ErPart {
        part_with_report(
            self.0.into(),
            type_name::<E>(),
            self.1.into_er_report_string(),
        )
    }
}
// A Box<dyn Error> can go in the tree even though the box itself doesn't implement Error.
macro_rules! boxed_report {
    ($error:ty) => {
        impl<R: ErReportText> IntoErPart<ErWithReport<$error>> for ($error, R) {
            type Error = $error;

            fn er_error(&self) -> &Self::Error {
                &self.0
            }

            #[track_caller]
            fn into_er_part(self) -> ErPart {
                part_with_report(
                    self.0,
                    type_name::<$error>(),
                    self.1.into_er_report_string(),
                )
            }
        }
    };
}
boxed_report!(BoxError);
#[cfg(er_unsync)]
boxed_report!(Box<dyn Error + Send + Sync>);
#[cfg(er_unsync)]
boxed_report!(Box<dyn Error + Send>);
#[cfg(er_unsync)]
boxed_report!(Box<dyn Error + Sync>);

#[track_caller]
fn part_with_report(error: BoxError, title: &'static str, report: Option<String>) -> ErPart {
    ErPart {
        saved: saved_report_list(title, report),
        ..error.into_er_part()
    }
}
impl<E: IntoErPart<Input>, Input, Mode> ErErrorContextExt<Mode, Input> for ErErrorWithSaved<E> {}
impl<E: IntoErPart<Input>, Input> ErErrorExt<Input> for ErErrorWithSaved<E> {}
impl<E> ErErrorWithSaved<E> {
    /// Build the new top from fields taken from the saved error, or from the top of a saved tree.
    ///
    /// `ErErrorExt` and `IntoErTree` both offer this for a saved tree, this one picks for you.
    #[track_caller]
    pub fn er_with<A, Input, Mode>(
        self,
        fields: impl ErWithFields<<Self as IntoErPart<Input>>::Error, A, Mode>,
    ) -> ErTree<A>
    where
        Self: ErErrorExt<Input>,
        A: Error + 'static,
    {
        ErErrorExt::er_with(self, fields)
    }
}

/// So a library's `(Error, String)` prints and unwraps like an er result, `start().er_unwrap_report()`.
impl<E: ErReportError<Input>, R: ErReportText, Input> IntoErSnapshot<Input> for (E, R) {
    fn into_er_snapshot(self) -> ErSnapshot {
        pair_snapshot(
            self.0.as_error(),
            type_name::<E>(),
            self.1.into_er_report_string(),
        )
    }

    fn into_er_report_text(self) -> String {
        let (error, report) = self;
        pair_report(&error, report.into_er_report_string())
    }
}

/// The same for a pair you keep around, `result.as_ref().er_report()`.
impl<E: ErReportError<Input>, R: ErReportText, Input> IntoErSnapshot<Input> for &(E, R) {
    fn into_er_snapshot(self) -> ErSnapshot {
        pair_snapshot(
            self.0.as_error(),
            type_name::<E>(),
            self.1.er_report_str().map(str::to_owned),
        )
    }

    fn into_er_report_text(self) -> String {
        match self.1.er_report_str() {
            Some(report) => report.to_owned(),
            None => report_string(self.0.as_error()),
        }
    }
}

#[doc(hidden)]
pub trait ErSavedPayload<Kind> {
    type Value;

    fn into_saved_payload(self) -> (Self::Value, ErSavedList);
}
impl<E, R: ErReportText> ErSavedPayload<ErWithReport> for (E, R) {
    type Value = (E, Option<String>);

    fn into_saved_payload(self) -> (Self::Value, ErSavedList) {
        let (error, report) = self;
        (
            (error, report.into_er_report_string()),
            ErSavedList::default(),
        )
    }
}
impl<P: ErSavedPayload<Kind>, Kind> ErSavedPayload<Kind> for ErErrorWithSaved<P> {
    type Value = P::Value;

    fn into_saved_payload(self) -> (Self::Value, ErSavedList) {
        let (value, mut saved) = self.value.into_saved_payload();
        let mut extra = self.saved;
        saved.append(&mut extra);

        (value, saved)
    }
}

impl<T, P, E> ErPresentationExt<ErWithReport> for Result<T, ErErrorWithSaved<P>>
where
    P: ErSavedPayload<ErWithReport, Value = (E, Option<String>)>,
{
    type Ok = T;
    type Err = E;

    #[track_caller]
    fn er_tree(self) -> ErResult<T, E> {
        match self {
            Ok(value) => Ok(value),
            Err(error) => {
                let (pair, saved) = error.into_saved_payload();
                Err(report_pair_tree(pair, saved))
            }
        }
    }

    fn er_report<Input>(self) -> Result<T, String>
    where
        E: ErReportError<Input>,
    {
        self.er_split_with(|(_, report)| report)
    }

    fn er_snapshot<Input>(self) -> Result<T, ErSnapshot>
    where
        E: ErReportError<Input>,
    {
        self.map_err(|error| {
            let ((error, report), saved) = error.into_saved_payload();
            let snapshot = pair_snapshot(error.as_error(), type_name::<E>(), report);
            snapshot_with_saved(snapshot, &saved)
        })
    }

    #[track_caller]
    fn er_split_with<A, Input>(self, into: impl FnOnce((E, String)) -> A) -> Result<T, A>
    where
        E: ErReportError<Input>,
    {
        self.map_err(|error| {
            let ((error, report), saved) = error.into_saved_payload();
            let report = pair_report(&error, report);
            into((error, append_saved_report(report, &saved)))
        })
    }
}
// A snapshot, or something holding one, keeps printing through any number of `.er_save()` calls.
macro_rules! saved_snapshot {
    ($($holder:ty),+) => {
        $(
            impl ErSavedPayload<ErSavedSnapshot> for $holder {
                type Value = Self;

                fn into_saved_payload(self) -> (Self, ErSavedList) {
                    (self, ErSavedList::default())
                }
            }
        )+
    };
}
saved_snapshot!(
    ErSnapshot,
    &ErSnapshot,
    &mut ErSnapshot,
    Box<ErSnapshot>,
    Rc<ErSnapshot>
);
#[cfg(target_has_atomic = "ptr")]
saved_snapshot!(Arc<ErSnapshot>);
#[cfg(any(er_unsync, target_has_atomic = "ptr"))]
saved_snapshot!(ErShared<ErSnapshot>);

impl<P, Input> IntoErSnapshot<(ErSavedSnapshot, Input)> for ErErrorWithSaved<P>
where
    P: ErSavedPayload<ErSavedSnapshot>,
    P::Value: IntoErSnapshot<Input>,
{
    fn into_er_snapshot(self) -> ErSnapshot {
        let (value, saved) = self.into_saved_payload();
        snapshot_with_saved(value.into_er_snapshot(), &saved)
    }

    fn into_er_report_text(self) -> String {
        let (value, saved) = self.into_saved_payload();
        value.into_er_report_with_saved(saved)
    }

    fn into_er_report_with_saved(self, mut extra: ErSavedList) -> String {
        let (value, mut saved) = self.into_saved_payload();
        saved.append(&mut extra);
        value.into_er_report_with_saved(saved)
    }
}
impl<'a, E, Input> IntoErSnapshot<Input> for &'a ErErrorWithSaved<E>
where
    &'a E: IntoErSnapshot<Input>,
{
    fn into_er_snapshot(self) -> ErSnapshot {
        snapshot_with_saved((&self.value).into_er_snapshot(), &self.saved)
    }

    fn into_er_report_text(self) -> String {
        self.into_er_report_with_saved(ErSavedList::default())
    }

    fn into_er_report_with_saved(self, mut extra: ErSavedList) -> String {
        let mut saved = self.saved.clone();
        saved.append(&mut extra);
        (&self.value).into_er_report_with_saved(saved)
    }
}

impl<P, E, Input> IntoErSnapshot<(ErSavedTree, Input)> for ErErrorWithSaved<P>
where
    P: ErSavedPayload<ErSavedTree>,
    P::Value: Deref<Target = ErTree<E>>,
    E: ErReportError<Input>,
{
    fn into_er_snapshot(self) -> ErSnapshot {
        let (tree, saved) = self.into_saved_payload();
        snapshot_with_saved(tree.er_snapshot(), &saved)
    }

    fn into_er_report_text(self) -> String {
        let (tree, saved) = self.into_saved_payload();
        report_with_saved(&tree, saved)
    }

    fn into_er_report_with_saved(self, mut extra: ErSavedList) -> String {
        let (tree, mut saved) = self.into_saved_payload();
        saved.append(&mut extra);
        report_with_saved(&tree, saved)
    }
}
macro_rules! saved_tree_view {
    (with_payload $tree:ty) => {
        impl<E> ErSavedPayload<ErSavedTree> for $tree {
            type Value = Self;

            fn into_saved_payload(self) -> (Self, ErSavedList) {
                (self, ErSavedList::default())
            }
        }

        saved_tree_view!($tree);
    };
    ($tree:ty) => {
        impl<E: ErReportError<Input>, Input> IntoErSnapshot<(ErSavedTree, Input)>
            for &ErErrorWithSaved<$tree>
        {
            fn into_er_snapshot(self) -> ErSnapshot {
                snapshot_with_saved(self.value.er_snapshot(), &self.saved)
            }

            fn into_er_report_text(self) -> String {
                report_with_saved(&self.value, self.saved.clone())
            }

            fn into_er_report_with_saved(self, mut extra: ErSavedList) -> String {
                let mut saved = self.saved.clone();
                saved.append(&mut extra);
                report_with_saved(&self.value, saved)
            }
        }
    };
}
saved_tree_view!(ErTree<E>);
saved_tree_view!(Box<ErTree<E>>);
saved_tree_view!(with_payload &ErTree<E>);
saved_tree_view!(with_payload &mut ErTree<E>);
saved_tree_view!(with_payload Rc<ErTree<E>>);
#[cfg(target_has_atomic = "ptr")]
saved_tree_view!(with_payload Arc<ErTree<E>>);
#[cfg(any(er_unsync, target_has_atomic = "ptr"))]
saved_tree_view!(with_payload ErSharedTree<E>);

fn report_with_saved<E: ErReportError<Input>, Input>(
    tree: &ErTree<E>,
    extra: ErSavedList,
) -> String {
    let mut report = tree.internal_report();
    report.extra = &extra;
    report.render_string()
}

fn snapshot_with_saved(mut snapshot: ErSnapshot, saved: &[ErSaved]) -> ErSnapshot {
    snapshot.saved.extend(snapshot_saved(saved));
    snapshot
}

fn pair_report<E: ErReportError<Input>, Input>(error: &E, report: Option<String>) -> String {
    report.unwrap_or_else(|| report_string(error.as_error()))
}

pub fn append_saved_report(mut report: String, saved: &[ErSaved]) -> String {
    if write_saved_parts(&mut report, saved, &[]).is_err() {
        report.clear();
        report.push_str("ER_FMT_FAILED");
    }

    report
}

fn saved_report(title: &'static str, report: String) -> ErSaved {
    ErSaved {
        title: title.into(),
        kind: ErSavedKind::Text(report),
    }
}

fn saved_report_list(title: &'static str, report: Option<String>) -> ErSavedList {
    let entries = report.map(|report| Shared::new(alloc::vec![saved_report(title, report)]));
    ErSavedList(entries)
}

fn pair_snapshot(
    error: &(dyn Error + 'static),
    title: &'static str,
    report: Option<String>,
) -> ErSnapshot {
    let mut snapshot = ErSnapshot::from(ErEntries::new(error, &[], None));

    if let Some(report) = report {
        snapshot.saved = alloc::vec![saved_report(title, report)];
    }

    snapshot
}

#[track_caller]
fn report_pair_tree<E>((top, report): (E, Option<String>), mut saved: ErSavedList) -> ErTree<E> {
    let mut tree = ErTree::from_top(top);
    tree.saved = saved_report_list(type_name::<E>(), report);
    tree.saved.append(&mut saved);
    tree
}

impl<E, R, Input> ErErrorExt<ErWithReport<Input>> for (E, R) where
    Self: IntoErPart<ErWithReport<Input>>
{
}
#[cfg(any(er_unsync, target_has_atomic = "ptr"))]
impl<E: ErReportError<Input>, R: ErReportText, Input> ErShareReportExt<Input> for (E, R) {
    type Shared = ErSharedTree<E>;

    #[track_caller]
    fn er_share(self) -> Self::Shared {
        ErErrorWithSaved::new(self).er_share()
    }
}
#[cfg(any(er_unsync, target_has_atomic = "ptr"))]
impl<T, E, R, Input> ErShareReportExt<Input> for Result<T, (E, R)>
where
    (E, R): ErShareReportExt<Input>,
{
    type Shared = Result<T, <(E, R) as ErShareReportExt<Input>>::Shared>;

    #[track_caller]
    fn er_share(self) -> Self::Shared {
        match self {
            Ok(value) => Ok(value),
            Err(pair) => Err(pair.er_share()),
        }
    }
}
impl<E, R, Input, Mode> ErErrorContextExt<Mode, ErWithReport<Input>> for (E, R) where
    Self: IntoErPart<ErWithReport<Input>>
{
}
impl<E: IntoErPart<Input>, Input> IntoErPart<Input> for ErErrorWithSaved<E> {
    type Error = E::Error;

    fn er_error(&self) -> &E::Error {
        self.value.er_error()
    }

    #[track_caller]
    fn into_er_part(self) -> ErPart {
        let ErErrorWithSaved { value, mut saved } = self;
        let mut part = value.into_er_part();
        part.saved.append(&mut saved);
        part
    }
}

impl<E: Error> ErSaveExt<ErReady> for E {
    type Value = E;

    fn er_save(self, title: impl Into<String>, saved: impl ErSaveInput) -> ErErrorWithSaved<E> {
        ErErrorWithSaved::new(self).saving(title, saved)
    }
}
impl<E: ErReportError<ErBoxedInput>> ErSaveExt<ErBoxedInput> for E {
    type Value = E;

    fn er_save(self, title: impl Into<String>, saved: impl ErSaveInput) -> ErErrorWithSaved<E> {
        ErErrorWithSaved::new(self).saving(title, saved)
    }
}
impl<E> ErSaveExt for ErErrorWithSaved<E> {
    type Value = E;

    fn er_save(self, title: impl Into<String>, saved: impl ErSaveInput) -> Self {
        self.saving(title, saved)
    }
}
impl ErSaveExt for () {
    type Value = ();

    fn er_save(self, title: impl Into<String>, saved: impl ErSaveInput) -> ErErrorWithSaved<()> {
        ErErrorWithSaved::new(self).saving(title, saved)
    }
}
impl<A> ErSaveExt for (A,) {
    type Value = A;

    fn er_save(self, title: impl Into<String>, saved: impl ErSaveInput) -> ErErrorWithSaved<A> {
        ErErrorWithSaved::new(self.0).saving(title, saved)
    }
}
macro_rules! save_fields {
    ($($name:ident),+) => {
        impl<$($name),+> ErSaveExt for ($($name),+) {
            type Value = Self;

            fn er_save(self, title: impl Into<String>, saved: impl ErSaveInput) -> ErErrorWithSaved<Self> {
                ErErrorWithSaved::new(self).saving(title, saved)
            }
        }
    };
}
save_fields!(A, B);
save_fields!(A, B, C);
save_fields!(A, B, C, D);
save_fields!(A, B, C, D, F);
save_fields!(A, B, C, D, F, G);
save_fields!(A, B, C, D, F, G, H);
save_fields!(A, B, C, D, F, G, H, I);
save_fields!(A, B, C, D, F, G, H, I, J);
save_fields!(A, B, C, D, F, G, H, I, J, K);
save_fields!(A, B, C, D, F, G, H, I, J, K, L);
save_fields!(A, B, C, D, F, G, H, I, J, K, L, M);

/// Each saved report below the tree under its title, skipping ones already printed.
/// `extra` continues the same list, so a tree's reports and the ones saved on its Result print together
/// without copying either.
pub fn write_saved_parts<W: fmt::Write + ?Sized>(
    writer: &mut W,
    saved: &[ErSaved],
    extra: &[ErSaved],
) -> fmt::Result {
    if extra.is_empty() {
        return write_saved(writer, saved);
    }

    let all: Vec<&ErSaved> = saved.iter().chain(extra).collect();
    write_saved(writer, &all)
}

fn write_saved<W: fmt::Write + ?Sized, T: Borrow<ErSaved>>(
    writer: &mut W,
    saved: &[T],
) -> fmt::Result {
    #[cfg(feature = "stack_traces")]
    if let Some(rendered) = trace_text(saved) {
        return write_distinct_saved(writer, &rendered);
    }

    write_distinct_saved(writer, saved)
}

// Below this a plain scan beats building an index, and almost every report is below it.
const SCAN_LIMIT: usize = 16;

fn write_distinct_saved<W: fmt::Write + ?Sized, T: Borrow<ErSaved>>(
    writer: &mut W,
    saved: &[T],
) -> fmt::Result {
    if saved.len() <= SCAN_LIMIT {
        for (index, entry) in saved.iter().enumerate() {
            let entry = entry.borrow();
            if saved[..index]
                .iter()
                .any(|earlier| earlier.borrow() == entry)
            {
                continue;
            }

            write!(writer, "\n\n{entry}")?;
        }

        return Ok(());
    }

    for (entry, repeated) in saved.iter().zip(repeated(saved)) {
        if !repeated {
            write!(writer, "\n\n{}", entry.borrow())?;
        }
    }

    Ok(())
}

/// Which entries equal one before them. A batch of thousands of reports would spend seconds comparing each one
/// against all the earlier ones, so this groups them by a hash first and only compares inside a group.
fn repeated<T: Borrow<ErSaved>>(saved: &[T]) -> Vec<bool> {
    let mut order: Vec<_> = saved
        .iter()
        .enumerate()
        .map(|(index, entry)| (saved_hash(entry.borrow()), index))
        .collect();
    order.sort_unstable();

    let mut repeated = alloc::vec![false; saved.len()];
    let mut start = 0;
    while start < order.len() {
        let hash = order[start].0;
        let group = order[start..]
            .iter()
            .take_while(|(other, _)| *other == hash)
            .count();
        let group = &order[start..start + group];

        for (position, &(_, index)) in group.iter().enumerate().skip(1) {
            repeated[index] = group[..position].iter().any(|&(_, earlier)| {
                !repeated[earlier] && saved[earlier].borrow() == saved[index].borrow()
            });
        }

        start += group.len();
    }

    repeated
}

struct Fnv(u64);
impl Hasher for Fnv {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.0 = (self.0 ^ u64::from(*byte)).wrapping_mul(0x0100_0000_01b3);
        }
    }
}

/// A trace is the same as another when it's the same capture. That stops working where one title holds a trace
/// next to text or next to a different capture, which is a snapshot's text copy of a trace meeting a live one.
/// Only the traces under such a title get turned into text to compare, `None` when there aren't any.
#[cfg(feature = "stack_traces")]
fn trace_text<T: Borrow<ErSaved>>(saved: &[T]) -> Option<Vec<Cow<'_, ErSaved>>> {
    #[derive(Default)]
    struct Title {
        text: bool,
        capture: Option<*const Backtrace>,
        captures_differ: bool,
    }

    let has_trace = |entry: &T| match entry.borrow().kind {
        ErSavedKind::StackTrace(_) => true,
        ErSavedKind::Text(_) | ErSavedKind::Snapshot(_) => false,
    };
    if !saved.iter().any(has_trace) {
        return None;
    }

    let mut titles: HashMap<&str, Title> = HashMap::new();
    for entry in saved {
        let entry = entry.borrow();

        match &entry.kind {
            ErSavedKind::Text(_) => titles.entry(&entry.title).or_default().text = true,
            ErSavedKind::StackTrace(trace) => {
                let title = titles.entry(&entry.title).or_default();
                let capture = Arc::as_ptr(trace);

                match title.capture {
                    Some(first) => title.captures_differ |= first != capture,
                    None => title.capture = Some(capture),
                }
            }
            ErSavedKind::Snapshot(_) => {}
        }
    }
    titles.retain(|_, title| title.capture.is_some() && (title.text || title.captures_differ));
    if titles.is_empty() {
        return None;
    }

    // One capture gets rendered once, whatever number of entries and titles point at it.
    let mut texts: HashMap<*const Backtrace, String> = HashMap::new();
    let mut seen = HashSet::new();
    let mut rendered = Vec::with_capacity(saved.len());
    for entry in saved {
        let entry = entry.borrow();
        let ErSavedKind::StackTrace(trace) = &entry.kind else {
            rendered.push(Cow::Borrowed(entry));
            continue;
        };
        if !titles.contains_key(entry.title.as_str()) {
            rendered.push(Cow::Borrowed(entry));
            continue;
        }

        let capture = Arc::as_ptr(trace);
        if !seen.insert((entry.title.as_str(), capture)) {
            continue;
        }

        let text = texts.entry(capture).or_insert_with(|| trace.to_string());
        rendered.push(Cow::Owned(ErSaved {
            title: entry.title.clone(),
            kind: ErSavedKind::Text(text.clone()),
        }));
    }

    Some(rendered)
}

/// The saved reports a snapshot keeps, with stack traces as text.
pub fn snapshot_saved(saved: &[ErSaved]) -> Vec<ErSaved> {
    #[cfg(feature = "stack_traces")]
    {
        // Keep the first shared trace inline so its copies need no cache allocation.
        let mut first_trace = None;
        let mut other_traces = None;
        let mut snapshot = Vec::with_capacity(saved.len());

        for (index, entry) in saved.iter().enumerate() {
            let ErSavedKind::StackTrace(trace) = &entry.kind else {
                snapshot.push(snapshot_entry(entry));
                continue;
            };

            let previous = if is_shared_capture(trace) {
                let identity = Arc::as_ptr(trace);
                match first_trace {
                    Some((first, previous)) if first == identity => Some(previous),
                    Some(_) => other_traces
                        .get_or_insert_with(HashMap::new)
                        .insert(identity, index),
                    None => {
                        first_trace = Some((identity, index));
                        None
                    }
                }
            } else {
                None
            };
            let kind = match previous {
                Some(previous) => snapshot[previous].kind.clone(),
                None => ErSavedKind::Text(trace.to_string()),
            };

            snapshot.push(ErSaved {
                title: entry.title.clone(),
                kind,
            });
        }

        snapshot
    }

    #[cfg(not(feature = "stack_traces"))]
    {
        saved.iter().map(snapshot_entry).collect()
    }
}

/// A snapshot saved inside a saved snapshot gets kept as its text. Code that saves the previous failure on every
/// new one would otherwise nest a level deeper each round, and printing, cloning or dropping that goes down
/// one stack frame per level until it runs out.
fn snapshot_entry(entry: &ErSaved) -> ErSaved {
    let ErSavedKind::Snapshot(snapshot) = &entry.kind else {
        return entry.clone();
    };
    let nested = |saved: &ErSaved| match saved.kind {
        ErSavedKind::Snapshot(_) => true,
        ErSavedKind::Text(_) => false,
        #[cfg(feature = "stack_traces")]
        ErSavedKind::StackTrace(_) => false,
    };
    if !snapshot.saved.iter().any(nested) {
        return entry.clone();
    }

    let saved = snapshot
        .saved
        .iter()
        .map(|saved| match &saved.kind {
            ErSavedKind::Snapshot(inner) => ErSaved {
                title: saved.title.clone(),
                kind: ErSavedKind::Text(inner.er_report()),
            },
            ErSavedKind::Text(_) => saved.clone(),
            #[cfg(feature = "stack_traces")]
            ErSavedKind::StackTrace(_) => saved.clone(),
        })
        .collect();

    ErSaved {
        title: entry.title.clone(),
        kind: ErSavedKind::Snapshot(ErSnapshot {
            entries: snapshot.entries.clone(),
            saved,
        }),
    }
}

#[cfg(feature = "stack_traces")]
fn is_shared_capture(trace: &Arc<Backtrace>) -> bool {
    trace.status() == BacktraceStatus::Captured && Arc::strong_count(trace) > 1
}
