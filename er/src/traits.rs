#[cfg(any(er_unsync, target_has_atomic = "ptr"))]
use crate::ErSharedTree;
use crate::aggregate::ErAggregateItems;
use crate::impls::result::{context, context_with, context_with_tree};
use crate::impls::saved::ReportText;
use crate::internal::{
    ErAllItem, ErInput, ErReportError, ErTreeContext, ErValFields, ErWithFields, er_make_cold,
};
use crate::{
    ErErrorWithSaved, ErFindAll, ErPart, ErResult, ErSavedList, ErSnapshot, ErTree,
    impls::tree::with_source,
};
use crate::{ErMake, ErSaveInput, ErSaveRef, ErTest};
use alloc::string::String;
use core::error::Error;

/// Add a new top error above an error or a tree.
pub trait ErErrorContextExt<Mode, Input = ()>: ErInput<Input> + Sized {
    /// Put your error above this one, keeping everything below it.
    #[track_caller]
    fn er<A>(self, error: impl ErMake<A, Mode>) -> ErTree<A>
    where
        A: Error + 'static,
    {
        // The same split as `.er()` on a Result, this usually sits in a branch of a function that succeeds.
        if er_make_cold(&error) {
            context(error, self)
        } else {
            with_source(error.er_make(), self)
        }
    }
}

/// Build context using the old error.
pub trait ErErrorExt<Input = ()>: IntoErPart<Input> + Sized {
    /// Build the new top from this error. For an `(Error, String)` pair, the closure gets the error and the report is kept.
    ///
    /// `er_bail!(device.er_with(|err| err.code));`
    #[track_caller]
    fn er_with<A, Mode>(self, fields: impl ErWithFields<Self::Error, A, Mode>) -> ErTree<A>
    where
        A: Error + 'static,
    {
        if core::mem::needs_drop::<A>() {
            context_with(fields, self)
        } else {
            with_source(fields.er_with_fields(self.er_error()), self)
        }
    }
}

/// Print, save or share a plain error and its native sources.
pub trait ErErrorPresentationExt<Input = ()> {
    /// Share this error as an `ErSharedTree` so multiple places can keep it.
    #[cfg(any(er_unsync, target_has_atomic = "ptr"))]
    #[track_caller]
    fn er_share(self) -> ErSharedTree<Self>
    where
        Self: ErReportError<Input> + Sized,
    {
        ErTree::from_top(self).er_share()
    }

    /// The whole report as text.
    #[must_use]
    fn er_report(&self) -> String
    where
        Self: 'static;

    /// Write the report straight to a buffer. The writer can return `fmt::Error` to stop early.
    fn er_write_report(&self, writer: &mut (impl core::fmt::Write + ?Sized)) -> core::fmt::Result
    where
        Self: 'static;

    /// Save the messages and source structure.
    fn er_snapshot(&self) -> ErSnapshot
    where
        Self: 'static;
}

/// Add context to a Result or turn None into an error.
pub trait ErContextExt<Mode, Input = ()> {
    type Ok;

    /// On Err or None, add your error on top and keep any original errors underneath.
    ///
    /// ```rust,ignore
    /// result.er(())?; // Empty struct
    /// result.er(path)?; // One field
    /// result.er((machine, "connect"))?; // More fields
    /// result.er(|_| (path, user.name()))?; // Compute fields on failure
    /// result.er(|| EnumErr::variant_name(arg1))?; // Return the whole error
    /// ```
    ///
    /// When your return type picks the error, you can skip `.er::<MyErr>(..)`.
    /// Pass `Copy + Debug` inputs directly. Er converts them on failure, so a `&Path` can fill a `PathBuf`.
    #[track_caller]
    fn er<A>(self, error: impl ErMake<A, Mode>) -> ErResult<Self::Ok, A>
    where
        A: Error + 'static;
}

/// Other ways to work with a Result's error.
pub trait ErResultExt<Input = ()> {
    type Ok;
    type Err;

    /// Build the new top from the old error. On an ErResult, the closure gets its typed top.
    /// For an `(Error, String)` pair, the closure gets the error and Er saves the report.
    /// Return fields or the finished error.
    #[track_caller]
    fn er_with<A, Mode>(
        self,
        fields: impl ErWithFields<<Self::Err as ErInput<Input>>::Error, A, Mode>,
    ) -> ErResult<Self::Ok, A>
    where
        Self: Sized,
        A: Error + 'static,
        Self::Err: ErInput<Input>,
    {
        match self.er_result_input() {
            Ok(value) => Ok(value),
            // Same split as `.er()`. Building a String here made every successful call twice as slow.
            Err(source) if !core::mem::needs_drop::<A>() => {
                let error = fields.er_with_fields(source.er_input_error());
                Err(with_source(error, source))
            }
            Err(source) => Err(context_with(fields, source)),
        }
    }

    /// Like `.er_with()`, but the closure gets the tree on an ErResult.
    #[track_caller]
    fn er_with_tree<A, Mode>(
        self,
        fields: impl ErWithFields<Self::Err, A, Mode>,
    ) -> ErResult<Self::Ok, A>
    where
        Self: Sized,
        A: Error + 'static,
        Self::Err: ErInput<Input>,
    {
        match self.er_result_input() {
            Ok(value) => Ok(value),
            Err(source) if !core::mem::needs_drop::<A>() => {
                let error = fields.er_with_fields(&source);
                Err(with_source(error, source))
            }
            Err(source) => Err(context_with_tree(fields, source)),
        }
    }

    #[doc(hidden)]
    fn er_result_input(self) -> Result<Self::Ok, Self::Err>;
}

/// Replace a Result's error with a value of your own.
pub trait ErValueExt {
    type Ok;
    type Err;

    /// For values that can't go in the tree, like `Err(85)`.
    /// Takes ownership of the value. Return fields or the finished error.
    ///
    /// ! WARNING ! This drops the old error, including any tree or report it carried.
    /// Use `.er()` or `.er_with()` to keep it.
    ///
    /// ```rust,ignore
    /// device_status().er_val(|v| v)?;
    /// ```
    #[track_caller]
    fn er_val<A, Mode>(self, fields: impl ErValFields<Self::Err, A, Mode>) -> ErResult<Self::Ok, A>
    where
        Self: Sized,
        A: Error + 'static,
    {
        match self.er_value_input() {
            Ok(value) => Ok(value),
            // No cold split here, unlike `.er()`. This sits where a plain value becomes an error, and building it
            // in a cold call measured slower on success and on failure.
            Err(value) => Err(fields.er_val_fields(value).into_tree()),
        }
    }

    #[doc(hidden)]
    fn er_value_input(self) -> Result<Self::Ok, Self::Err>;
}

/// Collect successful values and add context to failures.
pub trait ErIteratorExt<Mode, Input = ()>: Sized {
    type Ok;

    /// Stop at the first error.
    #[track_caller]
    fn er_collect<C, A>(self, error: impl ErMake<A, Mode>) -> ErResult<C, A>
    where
        C: FromIterator<Self::Ok>,
        A: Error + 'static;

    /// Go through the whole iterator and keep every error, dropping the oks if anything fails.
    #[track_caller]
    fn er_collect_all<C, A>(self, error: impl ErMake<A, Mode>) -> ErResult<C, A>
    where
        C: FromIterator<Self::Ok>,
        A: Error + 'static;
}

/// Print, unwrap, split or share a Result holding a tree, a Wrap or an error with saved reports.
pub trait ErPresentationExt<Kind = ()> {
    type Ok;
    type Err;

    /// Get the normal Er result back.
    #[track_caller]
    fn er_tree(self) -> ErResult<Self::Ok, Self::Err>;

    /// The whole report as text, leaves Ok alone.
    #[track_caller]
    fn er_report<Input>(self) -> Result<Self::Ok, String>
    where
        Self: Sized,
        Self::Err: ErReportError<Input>,
    {
        match self.er_tree() {
            Ok(value) => Ok(value),
            Err(tree) => Err(tree.er_report()),
        }
    }

    /// Save the failed tree as messages, leaves Ok alone.
    #[track_caller]
    fn er_snapshot<Input>(self) -> Result<Self::Ok, ErSnapshot>
    where
        Self: Sized,
        Self::Err: ErReportError<Input>,
    {
        match self.er_tree() {
            Ok(value) => Ok(value),
            Err(tree) => Err(tree.er_snapshot()),
        }
    }

    /// Return the top error with its whole report. Leaves Ok alone.
    #[track_caller]
    fn er_split<Input>(self) -> Result<Self::Ok, (Self::Err, String)>
    where
        Self: Sized,
        Self::Err: ErReportError<Input>,
    {
        self.er_split_with(core::convert::identity)
    }

    /// On Err, give the top error and its report to the closure. Leaves Ok alone.
    #[track_caller]
    fn er_split_with<A, Input>(
        self,
        into: impl FnOnce((Self::Err, String)) -> A,
    ) -> Result<Self::Ok, A>
    where
        Self: Sized,
        Self::Err: ErReportError<Input>,
    {
        match self.er_tree() {
            Ok(value) => Ok(value),
            Err(tree) => Err(tree.er_split_with(into)),
        }
    }

    /// Freeze the failed tree so you can clone it and put it below other errors. Leaves Ok alone.
    #[cfg(any(er_unsync, target_has_atomic = "ptr"))]
    #[track_caller]
    fn er_share(self) -> Result<Self::Ok, ErSharedTree<Self::Err>>
    where
        Self: Sized,
    {
        match self.er_tree() {
            Ok(value) => Ok(value),
            Err(tree) => Err(tree.er_share()),
        }
    }

    /// Get the Ok value, or panic with the whole report.
    ///
    /// # Panics
    /// Panics if the result is Err.
    #[track_caller]
    fn er_unwrap<Input>(self) -> Self::Ok
    where
        Self: Sized,
        Self::Err: ErReportError<Input>,
    {
        self.er_report().map_err(ReportText).unwrap()
    }

    /// Get the Ok value, or panic with your message and the whole report.
    ///
    /// # Panics
    /// Panics if the result is Err.
    #[track_caller]
    fn er_expect<Input>(self, message: &str) -> Self::Ok
    where
        Self: Sized,
        Self::Err: ErReportError<Input>,
    {
        self.er_report().map_err(ReportText).expect(message)
    }
}

/// Print or unwrap a Result holding a snapshot, or a library's `(Error, String)`.
pub trait ErSnapshotPresentationExt<Input = ()> {
    type Ok;

    /// The whole saved report as text, leaves Ok alone.
    fn er_report(self) -> Result<Self::Ok, String>;

    /// Get the saved entries back, leaves Ok alone.
    fn er_snapshot(self) -> Result<Self::Ok, ErSnapshot>;

    /// Get the Ok value, or panic with the whole saved report.
    ///
    /// # Panics
    /// Panics if the result is Err.
    #[track_caller]
    fn er_unwrap(self) -> Self::Ok
    where
        Self: Sized,
    {
        self.er_report().map_err(ReportText).unwrap()
    }

    /// Get the Ok value, or panic with your message and the whole saved report.
    ///
    /// # Panics
    /// Panics if the result is Err.
    #[track_caller]
    fn er_expect(self, message: &str) -> Self::Ok
    where
        Self: Sized,
    {
        self.er_report().map_err(ReportText).expect(message)
    }
}

/// Keep a library's error and report together when several places need the same failure.
#[cfg(any(er_unsync, target_has_atomic = "ptr"))]
pub trait ErShareReportExt<Input = ()> {
    type Shared;

    #[track_caller]
    fn er_share(self) -> Self::Shared;
}

/// Add context using a shared tree or a reference to it.
#[cfg(any(er_unsync, target_has_atomic = "ptr"))]
pub trait ErSharedTreeRefExt<E, Input = ()> {
    /// Build a new top from the whole shared tree and keep that tree as its cause.
    #[track_caller]
    fn er_with_tree<A, Mode>(
        self,
        fields: impl ErWithFields<ErSharedTree<E>, A, Mode>,
    ) -> ErTree<A>
    where
        A: Error + 'static;
}

/// Find concrete errors in a tree whose top is a boxed error.
pub trait ErBoxedTreeExt {
    /// Returns the first match, including the boxed top and native sources.
    fn er_find<T: Error + 'static>(&self) -> Option<&T>;

    /// Finds all the instances of an error type.
    fn er_find_all<T: Error + 'static>(&self) -> ErFindAll<'_, T>;

    /// True if this type is anywhere in the tree or its native sources.
    fn er_contains<T: Error + 'static>(&self) -> bool {
        self.er_find::<T>().is_some()
    }
}

/// Print or unwrap a Result that borrows a tree.
pub trait ErBorrowedPresentationExt<'a> {
    type Ok;
    type Err: 'a;

    /// Get the borrowed tree back, leaves Ok alone.
    fn er_tree(self) -> Result<Self::Ok, &'a ErTree<Self::Err>>;

    /// The whole report as text, leaves Ok alone.
    fn er_report<Input>(self) -> Result<Self::Ok, String>
    where
        Self::Err: ErReportError<Input>;

    /// Save the failed tree as messages, leaves Ok alone.
    fn er_snapshot<Input>(self) -> Result<Self::Ok, ErSnapshot>
    where
        Self::Err: ErReportError<Input>;

    /// Get the Ok value, or panic with the whole report.
    ///
    /// # Panics
    /// Panics if the result is Err.
    #[track_caller]
    fn er_unwrap<Input>(self) -> Self::Ok
    where
        Self: Sized,
        Self::Err: ErReportError<Input>,
    {
        self.er_report().map_err(ReportText).unwrap()
    }

    /// Get the Ok value, or panic with your message and the whole report.
    ///
    /// # Panics
    /// Panics if the result is Err.
    #[track_caller]
    fn er_expect<Input>(self, message: &str) -> Self::Ok
    where
        Self: Sized,
        Self::Err: ErReportError<Input>,
    {
        self.er_report().map_err(ReportText).expect(message)
    }
}

/// Borrow a tree from `&ErTree`, `&Arc<ErTree>`, etc.
pub trait IntoErTreeRef<'a>: Sized {
    type Error: 'a;

    /// Borrow the tree.
    fn into_er_tree_ref(self) -> &'a ErTree<Self::Error>;
}

/// Anything that can become a snapshot, a snapshot itself or a library's `(Error, String)`.
pub trait IntoErSnapshot<Input = ()> {
    /// Take the saved entries out. Borrowed snapshots copy their entries.
    fn into_er_snapshot(self) -> ErSnapshot;

    /// The report as text. A library's `(Error, String)` already has one, so it hands that back.
    #[doc(hidden)]
    fn into_er_report_text(self) -> String
    where
        Self: Sized,
    {
        self.into_er_snapshot().er_report()
    }

    #[doc(hidden)]
    fn into_er_report_with_saved(self, saved: ErSavedList) -> String
    where
        Self: Sized,
    {
        crate::impls::saved::append_saved_report(self.into_er_report_text(), &saved)
    }
}

/// Turn an Option or Result into ErTest. Existing test failures pass through.
pub trait ErTestExt<Input = ()> {
    type Ok;

    /// None reports "Option was None" with this call's location.
    /// `Err(())` reports a test failure at this call too.
    #[track_caller]
    fn er_test(self) -> ErTest<Self::Ok>;
}

/// Turns an error or tree into a node and its metadata.
pub trait IntoErPart<Input = ()> {
    type Error: ?Sized;

    /// Borrow the error, or the top if this is a tree.
    fn er_error(&self) -> &Self::Error;

    /// Move the error and its saved reports into a subtree.
    #[track_caller]
    fn into_er_part(self) -> ErPart;
}

/// Get the tree out of a tree or a Wrap.
pub trait IntoErTree {
    type Error;

    /// Take the tree out, keeping its errors and saved reports.
    fn into_er_tree(self) -> ErTree<Self::Error>;

    /// Build the new top from fields taken from the old top.
    #[track_caller]
    fn er_with<A, Mode>(
        self,
        fields: impl ErWithFields<Self::Error, A, <ErTree<Self::Error> as ErTreeContext<Mode>>::Fields>,
    ) -> ErTree<A>
    where
        Self: Sized,
        ErTree<Self::Error>: ErTreeContext<Mode>,
        A: Error + 'static,
    {
        self.into_er_tree().er_with(fields)
    }

    /// Like `.er_with()`, but the closure gets the whole tree.
    #[track_caller]
    fn er_with_tree<A, Mode>(
        self,
        fields: impl ErWithFields<
            ErTree<Self::Error>,
            A,
            <ErTree<Self::Error> as ErTreeContext<Mode>>::Fields,
        >,
    ) -> ErTree<A>
    where
        Self: Sized,
        ErTree<Self::Error>: ErTreeContext<Mode>,
        A: Error + 'static,
    {
        self.into_er_tree().er_with_tree(fields)
    }

    /// Add an error or several failures below the current top. Successful Results are skipped.
    #[track_caller]
    fn er_add<Mode, Kind>(
        self,
        items: impl ErAggregateItems<Kind, Item: ErAllItem<Mode>>,
    ) -> ErTree<Self::Error>
    where
        Self: Sized,
    {
        self.into_er_tree().er_add(items)
    }

    /// Take the top error and its whole report out of the tree.
    #[must_use]
    #[track_caller]
    fn er_split<Input>(self) -> (Self::Error, String)
    where
        Self: Sized,
        Self::Error: ErReportError<Input>,
    {
        self.into_er_tree().er_split()
    }

    /// Give the top error and its report to the closure.
    #[track_caller]
    fn er_split_with<A, Input>(self, into: impl FnOnce((Self::Error, String)) -> A) -> A
    where
        Self: Sized,
        Self::Error: ErReportError<Input>,
    {
        self.into_er_tree().er_split_with(into)
    }
}

/// Save a report on an error or on the fields you build it from.
///
/// A single field still goes in a tuple, `.er(|_| (path,).er_save(..))`.
pub trait ErSaveExt<Mode = ()>: Sized {
    type Value;

    /// `er_bail!(SyncErr::server(code).er_save("server", report))`
    ///
    /// `er_bail!((&path, "close database").er_save("server", report))`
    fn er_save(
        self,
        title: impl Into<String>,
        saved: impl ErSaveInput,
    ) -> ErErrorWithSaved<Self::Value>;
}

/// Save a report borrowed from a failed Result.
pub trait ErResultSaveExt: Sized {
    type Ok;
    type Err;

    /// The closure gets the whole error from the Result.
    ///
    /// `client.send(request).er_save("server", |err| &err.report).er(id)?;`
    fn er_save<S: ErSaveRef + ?Sized>(
        self,
        title: impl Into<String>,
        saved: impl FnOnce(&Self::Err) -> &S,
    ) -> Result<Self::Ok, ErErrorWithSaved<Self::Err>>;
}
