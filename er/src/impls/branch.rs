use crate::aggregate::{self, ErAggregateItems};
use crate::impls::saved::ErSavedPayload;
use crate::{
    ErAllItem, ErBranch, ErErrorWithSaved, ErMainFailure, ErReportError, ErSavedList,
    ErSavedSnapshot, ErSnapshot, ErTree, InternalReport, IntoErSnapshot, IntoErTree,
};
use alloc::{string::String, vec::Vec};
use core::{error::Error, fmt};

impl<E: Error> From<E> for ErBranch<E> {
    fn from(value: E) -> Self {
        Self::from_error(value)
    }
}
impl<E> From<ErBranch<E>> for ErTree<E> {
    #[track_caller]
    fn from(branch: ErBranch<E>) -> Self {
        let ErErrorWithSaved {
            value,
            saved,
            nodes,
        } = branch;
        Self {
            saved,
            nodes,
            ..Self::from_top(value)
        }
    }
}
impl<E> IntoErTree for ErBranch<E> {
    type Error = E;

    #[track_caller]
    fn into_er_tree(self) -> ErTree<E> {
        self.into()
    }
}
impl<E: Error + 'static> From<ErBranch<E>> for (E, String) {
    #[track_caller]
    fn from(branch: ErBranch<E>) -> Self {
        ErTree::from(branch).into()
    }
}
impl<E> From<ErBranch<E>> for ErMainFailure<E> {
    #[track_caller]
    fn from(branch: ErBranch<E>) -> Self {
        Self {
            tree: branch.into(),
        }
    }
}

impl<E> ErBranch<E> {
    pub(crate) fn from_error(value: E) -> Self {
        Self {
            value,
            saved: ErSavedList::default(),
            nodes: Vec::new(),
        }
    }

    /// Add children in order. Successful Results are skipped.
    #[track_caller]
    pub fn er_add<Mode, Kind>(
        mut self,
        items: impl ErAggregateItems<Kind, Item: ErAllItem<Mode>>,
    ) -> Self {
        aggregate::append(&mut self.nodes, items);
        self
    }

    fn report<Input>(&self) -> InternalReport<'_>
    where
        E: ErReportError<Input>,
    {
        InternalReport {
            top: self.value.as_error(),
            nodes: &self.nodes,
            saved: &self.saved,
            extra: &[],
            src_location: None,
        }
    }

    /// Print the branch as it is, without recording a location.
    #[must_use]
    pub fn er_report<Input>(&self) -> String
    where
        E: ErReportError<Input>,
    {
        self.report().render_string()
    }

    /// Write this branch straight to a buffer.
    pub fn er_write_report<Input>(&self, mut writer: &mut (impl fmt::Write + ?Sized)) -> fmt::Result
    where
        E: ErReportError<Input>,
    {
        self.report().write_to(&mut writer)
    }

    /// Copy the branch with the locations its children already have.
    pub fn er_snapshot<Input>(&self) -> ErSnapshot
    where
        E: ErReportError<Input>,
    {
        ErSnapshot::capture(self.value.as_error(), &self.nodes, &self.saved, None)
    }
}

impl<E> ErSavedPayload<ErSavedSnapshot> for &ErBranch<E> {
    type Value = Self;

    fn into_saved_payload(self) -> (Self, ErSavedList) {
        (self, ErSavedList::default())
    }
}
impl<E> ErSavedPayload<ErSavedSnapshot> for &mut ErBranch<E> {
    type Value = Self;

    fn into_saved_payload(self) -> (Self, ErSavedList) {
        (self, ErSavedList::default())
    }
}
impl<E: ErReportError<Input>, Input> IntoErSnapshot<Input> for &ErBranch<E> {
    fn into_er_snapshot(self) -> ErSnapshot {
        self.er_snapshot()
    }

    fn into_er_report_text(self) -> String {
        self.er_report()
    }

    fn into_er_report_with_saved(self, extra: ErSavedList) -> String {
        InternalReport {
            extra: &extra,
            ..self.report()
        }
        .render_string()
    }
}
