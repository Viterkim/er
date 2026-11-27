use crate::aggregate::{self, ErAddRoot, ErAggregateItems};
use crate::impls::error::report_string;
use crate::render::LineWriter;
use crate::types::Shared;
use crate::{
    ErAllItem, ErBoxError, ErBranch, ErBuildFrom, ErEntries, ErErrorContextExt, ErErrorExt,
    ErErrorWithSaved, ErNode, ErPresentationExt, ErReady, ErReportText, ErResult, ErResultSaveExt,
    ErSaveExt, ErSaveInput, ErSaveRef, ErSaved, ErSavedKind, ErSavedList, ErSavedSnapshot,
    ErSavedTree, ErSnapshot, ErTree, ErWithReport, ErWithSaves, InternalReport, IntoErNode,
    IntoErSnapshot, IntoErTree,
};
use crate::{ErBoxedInput, ErReportError, ErWithFields};
#[cfg(any(er_unsync, target_has_atomic = "ptr"))]
use crate::{ErShareReportExt, ErShared, ErSharedTree};
use alloc::borrow::Cow;
#[cfg(feature = "stack_traces")]
use alloc::string::ToString;
#[cfg(target_has_atomic = "ptr")]
use alloc::sync::Arc;
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

        #[cfg(any(er_unsync, target_has_atomic = "ptr"))]
        if Shared::ptr_eq(entries, &incoming) {
            return;
        }

        let entries = saved_entries_mut(entries);
        #[cfg(any(er_unsync, target_has_atomic = "ptr"))]
        match Shared::try_unwrap(incoming) {
            Ok(mut owned) => entries.append(&mut owned),
            Err(shared) => entries.extend_from_slice(&shared),
        }
        #[cfg(not(any(er_unsync, target_has_atomic = "ptr")))]
        {
            let mut incoming = incoming;
            entries.append(&mut incoming);
        }
    }
}

fn saved_hash(entry: &ErSaved) -> u64 {
    let mut hasher = SavedHasher(0xcbf2_9ce4_8422_2325);
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
        core::ptr::eq(&self[..], &other[..]) || self[..] == other[..]
    }
}
impl Eq for ErSavedList {}
impl Hash for ErSavedList {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self[..].hash(state);
    }
}

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
// What `String::from_utf8_lossy` hands you, which is how most response bodies arrive.
impl From<Cow<'_, str>> for ErSavedKind {
    fn from(text: Cow<'_, str>) -> Self {
        Self::Text(text.into_owned())
    }
}
impl From<&Cow<'_, str>> for ErSavedKind {
    fn from(text: &Cow<'_, str>) -> Self {
        Self::Text(String::from(&**text))
    }
}
impl From<ErSnapshot> for ErSavedKind {
    fn from(snapshot: ErSnapshot) -> Self {
        Self::Snapshot(limit_snapshot(&snapshot, SNAPSHOT_LEVELS - 1).unwrap_or(snapshot))
    }
}
impl From<&ErSnapshot> for ErSavedKind {
    fn from(snapshot: &ErSnapshot) -> Self {
        Self::Snapshot(
            limit_snapshot(snapshot, SNAPSHOT_LEVELS - 1).unwrap_or_else(|| snapshot.clone()),
        )
    }
}
#[cfg(feature = "stack_traces")]
impl From<Backtrace> for ErSavedKind {
    fn from(trace: Backtrace) -> Self {
        Self::StackTrace(Arc::new(trace))
    }
}
// A trace you already hold gets shared, not captured again.
#[cfg(feature = "stack_traces")]
impl From<Arc<Backtrace>> for ErSavedKind {
    fn from(trace: Arc<Backtrace>) -> Self {
        Self::StackTrace(trace)
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
save_one!(String, Cow<'_, str>, ErSnapshot);

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
save_reference!(str, String, Cow<'_, str>, ErSnapshot);
#[cfg(feature = "stack_traces")]
save_one!(Backtrace, Arc<Backtrace>);

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
impl ErSaveRef for Cow<'_, str> {
    fn er_save_ref(&self, title: String, saved: &mut ErSavedList) {
        self.er_save_into(title, saved);
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
            nodes: [],
        }
    }

    /// Add children to the error you saved a report on.
    #[track_caller]
    pub fn er_add<Mode, Kind, Input>(
        self,
        items: impl ErAggregateItems<Kind, Item: ErAllItem<Mode>>,
    ) -> T::Root
    where
        T: ErSavedError<Input>,
    {
        let mut root = self.value.with_saved(self.saved);
        aggregate::append(root.child_nodes(), items);
        root
    }
}
impl<T, Nodes> ErErrorWithSaved<T, Nodes> {
    pub(crate) fn into_branch(self) -> ErBranch<T>
    where
        Nodes: Into<Vec<ErNode>>,
    {
        ErBranch {
            value: self.value,
            saved: self.saved,
            nodes: self.nodes.into(),
        }
    }

    fn saving(mut self, title: impl Into<String>, saved: impl ErSaveInput) -> Self {
        saved.er_save_into(title.into(), &mut self.saved);
        self
    }

    #[doc(hidden)]
    #[track_caller]
    pub fn build<A: ErBuildFrom<T, Form>, Form>(self) -> ErErrorWithSaved<A, Nodes> {
        ErErrorWithSaved {
            value: A::er_build_from(self.value),
            saved: self.saved,
            nodes: self.nodes,
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
            Err(error) => Err(save_on(error, title, saved)),
        }
    }
}

// Saving always builds a String. Built in the caller, it made each successful call 5 times as expensive.
#[cold]
#[inline(never)]
fn save_on<E, S: ErSaveRef + ?Sized>(
    error: E,
    title: impl Into<String>,
    saved: impl FnOnce(&E) -> &S,
) -> ErErrorWithSaved<E> {
    let mut result = ErErrorWithSaved::new(error);
    saved(&result.value).er_save_ref(title.into(), &mut result.saved);

    result
}

impl<E, Nodes: Into<Vec<ErNode>>> ErErrorWithSaved<E, Nodes> {
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
        let mut tree = self.value.into_saved_tree(self.saved);
        crate::impls::node::append_nodes(&mut tree.nodes, &mut self.nodes.into());
        tree
    }
}

#[doc(hidden)]
pub trait ErSavedError<Input>: Sized {
    type Error;
    type Root: ErAddRoot<Error = Self::Error>;

    fn with_saved(self, saved: ErSavedList) -> Self::Root;

    #[track_caller]
    fn into_saved_tree(self, saved: ErSavedList) -> ErTree<Self::Error> {
        self.with_saved(saved).into_er_tree()
    }
}

macro_rules! save_plain_error {
    ($kind:ty, $presentation:ty, $bound:path) => {
        impl<E: $bound> ErSavedError<$kind> for E {
            type Error = E;
            type Root = ErBranch<E>;

            fn with_saved(self, saved: ErSavedList) -> Self::Root {
                ErBranch {
                    value: self,
                    saved,
                    nodes: Vec::new(),
                }
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
                    Err(ErErrorWithSaved { value, saved, .. }) => Err(value.into_saved_tree(saved)),
                }
            }
        }
    };
}
save_plain_error!((), ErWithSaves, Error);
save_plain_error!(
    ErBoxedInput,
    (ErWithSaves, ErBoxedInput),
    ErReportError<ErBoxedInput>
);
impl<E: ErReportError<Input>, R: ErReportText, Input> ErSavedError<ErWithReport<Input>> for (E, R) {
    type Error = E;
    type Root = ErBranch<E>;

    fn with_saved(self, saved: ErSavedList) -> Self::Root {
        report_pair_branch((self.0, self.1.into_er_report_string()), saved)
    }
}
impl<E> ErSavedError<ErSavedTree> for ErBranch<E> {
    type Error = E;
    type Root = Self;

    fn with_saved(mut self, mut saved: ErSavedList) -> Self {
        self.saved.append(&mut saved);
        self
    }
}
impl<E> ErSavedError<ErSavedTree> for ErTree<E> {
    type Error = E;
    type Root = Self;

    fn with_saved(mut self, mut saved: ErSavedList) -> Self {
        self.saved.append(&mut saved);
        self
    }
}
impl ErSavedError<ErSavedTree> for crate::ErTestFailure {
    type Error = crate::ErTestError;
    type Root = ErTree<Self::Error>;

    fn with_saved(self, saved: ErSavedList) -> Self::Root {
        self.into_er_tree().with_saved(saved)
    }
}
impl<E: ErSavedError<ErSavedTree>> ErSavedError<ErSavedTree> for Box<E> {
    type Error = E::Error;
    type Root = E::Root;

    fn with_saved(self, saved: ErSavedList) -> Self::Root {
        (*self).with_saved(saved)
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
    type Root = E::Root;

    fn with_saved(self, mut saved: ErSavedList) -> Self::Root {
        let Self {
            value,
            saved: mut previous,
            ..
        } = self;
        previous.append(&mut saved);
        value.with_saved(previous)
    }
}
impl<W: IntoErTree> From<ErErrorWithSaved<W>> for ErTree<W::Error> {
    #[track_caller]
    fn from(error: ErErrorWithSaved<W>) -> Self {
        let ErErrorWithSaved { value, saved, .. } = error;
        value.into_er_tree().into_saved_tree(saved)
    }
}
impl ErReportText for String {
    fn er_report_str(&self) -> Option<&str> {
        (!self.trim_start().is_empty()).then_some(self)
    }

    fn into_er_report_string(self) -> Option<String> {
        self.er_report_str().is_some().then_some(self)
    }
}
impl ErReportText for Option<String> {
    fn er_report_str(&self) -> Option<&str> {
        self.as_ref()?.er_report_str()
    }

    fn into_er_report_string(self) -> Option<String> {
        self?.into_er_report_string()
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
/// `.er()` puts the error in the tree and saves the report under the error's type, like `audio::StartError`.
/// An `Option<String>` report that's `None` just saves nothing.
impl<E: Error + Into<ErBoxError> + 'static, R: ErReportText> IntoErNode<ErWithReport> for (E, R) {
    type Error = E;

    fn er_error(&self) -> &E {
        &self.0
    }

    #[track_caller]
    fn into_er_node(self) -> ErNode {
        node_with_report(
            self.0.into(),
            type_name::<E>(),
            self.1.into_er_report_string(),
        )
    }
}
// A Box<dyn Error> can go in the tree even though the box itself doesn't implement Error.
macro_rules! boxed_report {
    ($error:ty) => {
        impl<R: ErReportText> IntoErNode<ErWithReport<$error>> for ($error, R) {
            type Error = $error;

            fn er_error(&self) -> &Self::Error {
                &self.0
            }

            #[track_caller]
            fn into_er_node(self) -> ErNode {
                node_with_report(
                    self.0,
                    type_name::<$error>(),
                    self.1.into_er_report_string(),
                )
            }
        }
    };
}
boxed_report!(ErBoxError);
#[cfg(er_unsync)]
boxed_report!(Box<dyn Error + Send + Sync>);
#[cfg(er_unsync)]
boxed_report!(Box<dyn Error + Send>);
#[cfg(er_unsync)]
boxed_report!(Box<dyn Error + Sync>);

#[track_caller]
fn node_with_report(error: ErBoxError, title: &'static str, report: Option<String>) -> ErNode {
    let mut node = error.into_er_node();
    node.saved = saved_report_list(title, report);
    node
}
impl<E: IntoErNode<Input>, Nodes: Into<Vec<ErNode>>, Input, Mode> ErErrorContextExt<Mode, Input>
    for ErErrorWithSaved<E, Nodes>
{
}
impl<E: IntoErNode<Input>, Nodes: Into<Vec<ErNode>>, Input> ErErrorExt<Input>
    for ErErrorWithSaved<E, Nodes>
{
}
impl<E, Nodes> ErErrorWithSaved<E, Nodes> {
    /// Build the new top from fields taken from the saved error, or from the top of a saved tree.
    ///
    /// `ErErrorExt` and `IntoErTree` both offer this for a saved tree, this one picks for you.
    #[track_caller]
    pub fn er_with<A, Input, Mode>(
        self,
        fields: impl ErWithFields<<Self as IntoErNode<Input>>::Error, A, Mode>,
    ) -> ErTree<A>
    where
        Self: ErErrorExt<Input>,
        A: Error + 'static,
    {
        ErErrorExt::er_with(self, fields)
    }
}

/// So a library's `(Error, String)` prints and unwraps like an er result, `start().er_unwrap()`.
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

    fn into_er_report_with_saved(self, saved: ErSavedList) -> String {
        let (error, report) = self;
        pair_report_with_saved(error.as_error(), report.into_er_report_string(), saved)
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

    fn into_er_report_with_saved(self, saved: ErSavedList) -> String {
        pair_report_with_saved(
            self.0.as_error(),
            self.1.er_report_str().map(str::to_owned),
            saved,
        )
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
                Err(report_pair_branch(pair, saved).into())
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
            let report = pair_report_with_saved(error.as_error(), report, saved);
            into((error, report))
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

// `stored.as_ref().er_save(..)` holds a borrowed pointer, which derefs to the pointer and not to the tree.
macro_rules! saved_tree_ref {
    ($holder:ty) => {
        impl<'a, E> ErSavedPayload<ErSavedTree> for &'a $holder {
            type Value = &'a ErTree<E>;

            fn into_saved_payload(self) -> (Self::Value, ErSavedList) {
                (&**self, ErSavedList::default())
            }
        }

        saved_tree_view!(&$holder);
    };
}
saved_tree_ref!(Rc<ErTree<E>>);
#[cfg(target_has_atomic = "ptr")]
saved_tree_ref!(Arc<ErTree<E>>);
#[cfg(any(er_unsync, target_has_atomic = "ptr"))]
saved_tree_ref!(ErSharedTree<E>);

fn report_with_saved<E: ErReportError<Input>, Input>(
    tree: &ErTree<E>,
    extra: ErSavedList,
) -> String {
    let mut report = tree.internal_report();
    report.extra = &extra;
    report.render_string()
}

fn snapshot_with_saved(mut snapshot: ErSnapshot, saved: &ErSavedList) -> ErSnapshot {
    if !saved.is_empty() {
        let owner = snapshot
            .entries
            .first_mut()
            .map_or(&mut snapshot.unattached, |entry| &mut entry.saved);
        *owner = SnapshotSaves::default().row(Some(owner), Some(saved));
    }

    snapshot
}

fn pair_report<E: ErReportError<Input>, Input>(error: &E, report: Option<String>) -> String {
    report.unwrap_or_else(|| report_string(error.as_error()))
}

fn pair_report_with_saved(
    error: &(dyn Error + 'static),
    report: Option<String>,
    saved: ErSavedList,
) -> String {
    if let Some(report) = report {
        return append_saved_report(report, &saved);
    }

    InternalReport {
        top: error,
        nodes: &[],
        saved: &saved,
        extra: &[],
        src_location: None,
    }
    .render_string()
}

pub fn append_saved_report(mut report: String, saved: &[ErSaved]) -> String {
    if saved.is_empty() {
        return report;
    }

    // A library's report can end with a line break or be empty, the saved sections bring their own spacing.
    let end = report.trim_end_matches(['\n', '\r']).len();
    report.truncate(end);

    if write_saved_parts(&mut report, saved, &[]).is_err() {
        report.clear();
        report.push_str("ER_FMT_FAILED");
    } else if end == 0 {
        report.drain(..report.len() - report.trim_start_matches('\n').len());
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
    let mut saved = ErSavedList::default();
    if let Some(report) = report {
        saved.push(saved_report(title, report));
    }

    saved
}

fn pair_snapshot(
    error: &(dyn Error + 'static),
    title: &'static str,
    report: Option<String>,
) -> ErSnapshot {
    let mut snapshot = ErSnapshot::from(ErEntries::new(error, &[], None));

    if let Some(report) = report
        && let Some(root) = snapshot.entries.first_mut()
    {
        root.saved.push(saved_report(title, report));
    }

    snapshot
}

fn report_pair_branch<E>(
    (value, report): (E, Option<String>),
    mut extra: ErSavedList,
) -> ErBranch<E> {
    let mut saved = saved_report_list(type_name::<E>(), report);
    saved.append(&mut extra);
    ErBranch {
        value,
        saved,
        nodes: Vec::new(),
    }
}

impl<E, R, Input> ErErrorExt<ErWithReport<Input>> for (E, R) where
    Self: IntoErNode<ErWithReport<Input>>
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
    Self: IntoErNode<ErWithReport<Input>>
{
}
impl<E: IntoErNode<Input>, Nodes: Into<Vec<ErNode>>, Input> IntoErNode<Input>
    for ErErrorWithSaved<E, Nodes>
{
    type Error = E::Error;

    fn er_error(&self) -> &E::Error {
        self.value.er_error()
    }

    #[track_caller]
    fn into_er_node(self) -> ErNode {
        let ErErrorWithSaved {
            value,
            mut saved,
            nodes,
        } = self;
        let mut node = value.into_er_node();
        node.saved.append(&mut saved);
        crate::impls::node::append_nodes(&mut node.nodes, &mut nodes.into());
        node
    }
}

impl<E: Error> ErSaveExt<ErReady> for E {
    type Value = E;
    type Nodes = Vec<ErNode>;

    fn er_save(self, title: impl Into<String>, saved: impl ErSaveInput) -> ErBranch<E> {
        ErBranch::from(self).saving(title, saved)
    }
}
impl<E: ErReportError<ErBoxedInput>> ErSaveExt<ErBoxedInput> for E {
    type Value = E;
    type Nodes = Vec<ErNode>;

    fn er_save(self, title: impl Into<String>, saved: impl ErSaveInput) -> ErBranch<E> {
        ErBranch::from_error(self).saving(title, saved)
    }
}
impl<E, Nodes> ErSaveExt for ErErrorWithSaved<E, Nodes> {
    type Value = E;
    type Nodes = Nodes;

    fn er_save(self, title: impl Into<String>, saved: impl ErSaveInput) -> Self {
        self.saving(title, saved)
    }
}
impl ErSaveExt for () {
    type Value = ();
    type Nodes = [ErNode; 0];

    fn er_save(self, title: impl Into<String>, saved: impl ErSaveInput) -> ErErrorWithSaved<()> {
        ErErrorWithSaved::new(self).saving(title, saved)
    }
}
impl<A> ErSaveExt for (A,) {
    type Value = A;
    type Nodes = [ErNode; 0];

    fn er_save(self, title: impl Into<String>, saved: impl ErSaveInput) -> ErErrorWithSaved<A> {
        ErErrorWithSaved::new(self.0).saving(title, saved)
    }
}
macro_rules! save_fields {
    ($($name:ident),+) => {
        impl<$($name),+> ErSaveExt for ($($name),+) {
            type Value = Self;
    type Nodes = [ErNode; 0];

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

/// Put attachments after an opaque report or snapshot that has no owning entries.
pub fn write_saved_parts<W: fmt::Write + ?Sized>(
    mut writer: &mut W,
    saved: &[ErSaved],
    extra: &[ErSaved],
) -> fmt::Result {
    if saved.is_empty() && extra.is_empty() {
        return Ok(());
    }

    let writer: &mut dyn fmt::Write = &mut writer;
    for_distinct_parts(saved, extra, |saved, _| write!(writer, "\n\n{saved}"))
}

pub(crate) fn for_distinct_parts(
    first: &[ErSaved],
    second: &[ErSaved],
    mut emit: impl FnMut(&ErSaved, bool) -> fmt::Result,
) -> fmt::Result {
    let saved = match (first, second) {
        ([], saved) | (saved, []) => saved,
        ([first], [second]) => return for_distinct_saved(&[first, second], emit),
        _ => {
            let both: Vec<_> = first.iter().chain(second).collect();
            return for_distinct_saved(&both, emit);
        }
    };

    match saved {
        [] => Ok(()),
        [only] => emit(only, true),
        _ => for_distinct_saved(saved, emit),
    }
}

fn for_distinct_saved<T: Borrow<ErSaved>>(
    saved: &[T],
    emit: impl FnMut(&ErSaved, bool) -> fmt::Result,
) -> fmt::Result {
    #[cfg(feature = "stack_traces")]
    if let Some(rendered) = trace_text(saved) {
        return emit_distinct_saved(&rendered, emit);
    }

    emit_distinct_saved(saved, emit)
}

// Below this a plain scan beats building an index, and almost every owner is below it.
const SCAN_LIMIT: usize = 16;

fn emit_distinct_saved<T: Borrow<ErSaved>>(
    saved: &[T],
    mut emit: impl FnMut(&ErSaved, bool) -> fmt::Result,
) -> fmt::Result {
    let mut distinct = saved
        .iter()
        .zip(duplicate_flags(saved))
        .filter_map(|(entry, repeated)| (!repeated).then_some(entry.borrow()))
        .peekable();
    while let Some(entry) = distinct.next() {
        emit(entry, distinct.peek().is_none())?;
    }

    Ok(())
}

fn duplicate_flags<T: Borrow<ErSaved>>(saved: &[T]) -> impl Iterator<Item = bool> {
    let duplicates = (saved.len() > SCAN_LIMIT).then(|| repeated(saved));
    saved
        .iter()
        .enumerate()
        .map(move |(index, entry)| match &duplicates {
            Some(duplicates) => duplicates[index],
            None => saved[..index]
                .iter()
                .any(|earlier| earlier.borrow() == entry.borrow()),
        })
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
    for group in order.chunk_by(|a, b| a.0 == b.0) {
        for (position, &(_, index)) in group.iter().enumerate().skip(1) {
            repeated[index] = group[..position].iter().any(|&(_, earlier)| {
                !repeated[earlier] && saved[earlier].borrow() == saved[index].borrow()
            });
        }
    }

    repeated
}

struct SavedHasher(u64);
impl Hasher for SavedHasher {
    fn finish(&self) -> u64 {
        self.0
    }

    #[inline(never)]
    fn write(&mut self, bytes: &[u8]) {
        let (words, tail) = bytes.as_chunks::<8>();
        for word in words {
            self.0 =
                (self.0.rotate_left(11) ^ u64::from_ne_bytes(*word)).wrapping_mul(0x0100_0000_01b3);
        }

        for byte in tail {
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

#[derive(Default)]
pub(crate) struct SnapshotSaves {
    #[cfg(feature = "stack_traces")]
    traces: Option<HashMap<*const Backtrace, String>>,
}
impl SnapshotSaves {
    pub fn row(&mut self, shared: Option<&ErSavedList>, own: Option<&ErSavedList>) -> ErSavedList {
        let saved = match (shared, own) {
            (Some(shared), Some(own)) if !own.is_empty() && !shared.is_empty() => {
                let mut both = shared.clone();
                both.append(&mut own.clone());
                both
            }
            (Some(shared), _) if !shared.is_empty() => shared.clone(),
            (_, Some(own)) => own.clone(),
            _ => return ErSavedList::default(),
        };

        #[cfg(feature = "stack_traces")]
        let mut saved = saved;
        #[cfg(feature = "stack_traces")]
        if saved
            .iter()
            .any(|entry| matches!(entry.kind, ErSavedKind::StackTrace(_)))
            && let Some(entries) = &mut saved.0
        {
            for entry in saved_entries_mut(entries) {
                if let ErSavedKind::StackTrace(trace) = &entry.kind {
                    let text = if is_shared_capture(trace) {
                        self.traces
                            .get_or_insert_with(HashMap::new)
                            .entry(Arc::as_ptr(trace))
                            .or_insert_with(|| trace.to_string())
                            .clone()
                    } else {
                        trace.to_string()
                    };
                    entry.kind = ErSavedKind::Text(text);
                }
            }
        }

        if saved.len() > 1 {
            let mut flags = duplicate_flags(&saved);
            if let Some(first) = flags.position(|repeated| repeated) {
                let mut unique = saved[..first].to_vec();
                unique.extend(
                    saved[first + 1..]
                        .iter()
                        .zip(flags)
                        .filter_map(|(entry, repeated)| (!repeated).then_some(entry))
                        .cloned(),
                );
                return ErSavedList(Some(Shared::new(unique)));
            }
        }

        saved
    }
}

// Saving the last retry on the next one can go on forever. Keep the older report as text
// before printing or dropping the history needs one stack frame per retry.
const SNAPSHOT_LEVELS: usize = 8;

fn limit_nested_saves(saved: &ErSavedList, levels: usize) -> Option<ErSavedList> {
    let mut changed = None;
    for (index, entry) in saved.iter().enumerate() {
        let ErSavedKind::Snapshot(inner) = &entry.kind else {
            continue;
        };
        let kind = if levels == 0 {
            ErSavedKind::Text(inner.er_report())
        } else if let Some(inner) = limit_snapshot(inner, levels - 1) {
            ErSavedKind::Snapshot(inner)
        } else {
            continue;
        };

        let entries = changed.get_or_insert_with(|| saved.to_vec());
        entries[index].kind = kind;
    }

    changed.map(|entries| ErSavedList(Some(Shared::new(entries))))
}

fn limit_snapshot(snapshot: &ErSnapshot, levels: usize) -> Option<ErSnapshot> {
    let mut changed = None;
    for (index, entry) in snapshot.entries.iter().enumerate() {
        if let Some(saved) = limit_nested_saves(&entry.saved, levels) {
            changed.get_or_insert_with(|| snapshot.clone()).entries[index].saved = saved;
        }
    }
    if let Some(saved) = limit_nested_saves(&snapshot.unattached, levels) {
        changed.get_or_insert_with(|| snapshot.clone()).unattached = saved;
    }

    changed
}

#[cfg(feature = "stack_traces")]
fn is_shared_capture(trace: &Arc<Backtrace>) -> bool {
    trace.status() == BacktraceStatus::Captured && Arc::strong_count(trace) > 1
}
