use crate::{
    ErBuilt, ErDefaults, ErFields, ErMake, ErReady, ErReport, ErSaved, ErSavedBox, ErValFields,
    ErWithFields,
};
use alloc::boxed::Box;

impl<A, F: FnOnce() -> A> ErMake<A, ErReady> for F {
    #[track_caller]
    fn er_make(self) -> A {
        self()
    }
}
impl<A: From<(P,)>, P, F: FnOnce(()) -> P> ErMake<A, ErFields> for F {
    #[track_caller]
    fn er_make(self) -> A {
        A::from((self(()),))
    }
}
impl<A: From<()>> ErMake<A, ErFields> for () {
    #[track_caller]
    fn er_make(self) -> A {
        A::from(())
    }
}
impl<E: ?Sized, A, Input, F> ErWithFields<E, A, ErFields> for F
where
    F: FnOnce(&E) -> Input,
    A: From<(Input,)>,
{
    type Input = Input;

    #[track_caller]
    fn er_with_fields(self, error: &E) -> A {
        A::from((self(error),))
    }
}
impl<E: ?Sized, A, F> ErWithFields<E, A, ErReady> for F
where
    F: FnOnce(&E) -> ErBuilt<A>,
{
    type Input = ErBuilt<A>;

    #[track_caller]
    fn er_with_fields(self, error: &E) -> A {
        self(error).0
    }
}
impl<E, A, Input, F> ErValFields<E, A, ErFields> for F
where
    F: FnOnce(E) -> Input,
    A: From<(Input,)>,
{
    type Input = Input;

    #[track_caller]
    fn er_val_fields(self, value: E) -> A {
        A::from((self(value),))
    }
}
impl<E, A, F> ErValFields<E, A, ErReady> for F
where
    F: FnOnce(E) -> ErBuilt<A>,
{
    type Input = ErBuilt<A>;

    #[track_caller]
    fn er_val_fields(self, value: E) -> A {
        self(value).0
    }
}

impl<A: From<(P, [(); 1])>, P, F: FnOnce(()) -> P> ErMake<A, ErDefaults> for F {
    #[track_caller]
    fn er_make(self) -> A {
        A::from((self(()), [()]))
    }
}
impl<E: ?Sized, A, Input, F> ErWithFields<E, A, ErDefaults> for F
where
    F: FnOnce(&E) -> Input,
    A: From<(Input, [(); 1])>,
{
    type Input = Input;

    #[track_caller]
    fn er_with_fields(self, error: &E) -> A {
        A::from((self(error), [()]))
    }
}
impl<E, A, Input, F> ErValFields<E, A, ErDefaults> for F
where
    F: FnOnce(E) -> Input,
    A: From<(Input, [(); 1])>,
{
    type Input = Input;

    #[track_caller]
    fn er_val_fields(self, value: E) -> A {
        A::from((self(value), [()]))
    }
}

macro_rules! saved_output {
    ($fields:ty, $tag:literal, $mode:ident, $target:ty, $build:expr) => {
        impl<E, A, F> ErValFields<ErReport<E>, $target, $mode<$fields, $tag>> for F
        where
            F: FnOnce(ErReport<E>) -> ErReport<E>,
            ErReport<E>: Into<($fields, [(); $tag])>,
            A: From<($fields, [(); $tag])>,
        {
            type Input = ErReport<E>;

            #[track_caller]
            fn er_val_fields(self, report: ErReport<E>) -> $target {
                $build(A::from(self(report).into()))
            }
        }

        impl<E, A, Input, F> ErValFields<ErReport<E>, $target, ($mode<$fields, $tag>, Input)> for F
        where
            F: FnOnce(ErReport<E>) -> (ErReport<E>, Input),
            ErReport<E>: Into<($fields, [(); $tag])>,
            A: From<(($fields, Input), [(); $tag])>,
        {
            type Input = (ErReport<E>, Input);

            #[track_caller]
            fn er_val_fields(self, report: ErReport<E>) -> $target {
                let (report, input) = self(report);
                let (fields, tag) = report.into();
                $build(A::from(((fields, input), tag)))
            }
        }

        impl<E, A, Input, F>
            ErValFields<ErReport<E>, $target, ($mode<$fields, $tag>, ErDefaults, Input)> for F
        where
            F: FnOnce(ErReport<E>) -> (ErReport<E>, Input),
            ErReport<E>: Into<($fields, [(); $tag])>,
            A: From<(($fields, ((Input, [(); 1]),)), [(); $tag])>,
        {
            type Input = (ErReport<E>, Input);

            #[track_caller]
            fn er_val_fields(self, report: ErReport<E>) -> $target {
                let (report, input) = self(report);
                let (fields, tag) = report.into();
                $build(A::from(((fields, ((input, [()]),)), tag)))
            }
        }
    };
}

pub(crate) use saved_output;

macro_rules! saved_outputs {
    ($mode:ident, $target:ty, $build:expr) => {
        $crate::impls::construct::saved_output!(
            (alloc::string::String,),
            4,
            $mode,
            $target,
            $build
        );
        $crate::impls::construct::saved_output!(
            (E, alloc::string::String,),
            5,
            $mode,
            $target,
            $build
        );
        $crate::impls::construct::saved_output!(
            (alloc::string::String,),
            6,
            $mode,
            $target,
            $build
        );
        $crate::impls::construct::saved_output!(
            (E, alloc::string::String,),
            7,
            $mode,
            $target,
            $build
        );
        $crate::impls::construct::saved_output!(
            (alloc::string::String, alloc::string::String,),
            8,
            $mode,
            $target,
            $build
        );
        $crate::impls::construct::saved_output!(
            (E, alloc::string::String, alloc::string::String,),
            9,
            $mode,
            $target,
            $build
        );
        $crate::impls::construct::saved_output!((crate::ErSnapshot,), 10, $mode, $target, $build);
        $crate::impls::construct::saved_output!(
            (E, crate::ErSnapshot,),
            11,
            $mode,
            $target,
            $build
        );
        $crate::impls::construct::saved_output!(
            (alloc::string::String, crate::ErSnapshot,),
            12,
            $mode,
            $target,
            $build
        );
        $crate::impls::construct::saved_output!(
            (E, alloc::string::String, crate::ErSnapshot,),
            13,
            $mode,
            $target,
            $build
        );
        $crate::impls::construct::saved_output!(
            (alloc::string::String, crate::ErSnapshot,),
            14,
            $mode,
            $target,
            $build
        );
        $crate::impls::construct::saved_output!(
            (E, alloc::string::String, crate::ErSnapshot,),
            15,
            $mode,
            $target,
            $build
        );
        $crate::impls::construct::saved_output!(
            (
                alloc::string::String,
                alloc::string::String,
                crate::ErSnapshot,
            ),
            16,
            $mode,
            $target,
            $build
        );
        $crate::impls::construct::saved_output!(
            (
                E,
                alloc::string::String,
                alloc::string::String,
                crate::ErSnapshot,
            ),
            17,
            $mode,
            $target,
            $build
        );
    };
}

#[cfg(any(er_unsync, target_has_atomic = "ptr"))]
pub(crate) use saved_outputs;

saved_outputs!(ErSaved, A, core::convert::identity);
saved_outputs!(ErSavedBox, Box<A>, Box::new);
