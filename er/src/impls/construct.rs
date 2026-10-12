use crate::{
    ErBuilt, ErFields, ErFromTree, ErMake, ErReady, ErReport, ErSaved, ErTree, ErValFields,
    ErWithFields,
};
use alloc::boxed::Box;

impl<A, F: FnOnce() -> A> ErMake<A, ErReady> for F {
    fn er_make(self) -> A {
        self()
    }
}
impl<A: From<(P,)>, P, F: FnOnce(()) -> P> ErMake<A, ErFields> for F {
    fn er_make(self) -> A {
        A::from((self(()),))
    }
}
impl<A: From<()>> ErMake<A, ErFields> for () {
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

    fn er_with_fields(self, error: &E) -> A {
        A::from((self(error),))
    }
}
impl<E: ?Sized, A, F> ErWithFields<E, A, ErReady> for F
where
    F: FnOnce(&E) -> ErBuilt<A>,
{
    type Input = ErBuilt<A>;

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

    fn er_val_fields(self, value: E) -> A {
        A::from((self(value),))
    }
}
impl<E, A, F> ErValFields<E, A, ErReady> for F
where
    F: FnOnce(E) -> ErBuilt<A>,
{
    type Input = ErBuilt<A>;

    fn er_val_fields(self, value: E) -> A {
        self(value).0
    }
}
impl<E, A, F> ErValFields<ErReport<E>, A, ErSaved> for F
where
    F: FnOnce(ErReport<E>) -> ErReport<E>,
    A: ErFromTree<E>,
{
    type Input = ErReport<E>;

    fn er_val_fields(self, report: ErReport<E>) -> A {
        A::er_from_report(self(report), ())
    }
}
impl<E, A, Input, F> ErValFields<ErReport<E>, A, (ErSaved, Input)> for F
where
    F: FnOnce(ErReport<E>) -> (ErReport<E>, Input),
    A: ErFromTree<E, Input>,
{
    type Input = (ErReport<E>, Input);

    fn er_val_fields(self, report: ErReport<E>) -> A {
        let (report, fields) = self(report);
        A::er_from_report(report, fields)
    }
}
impl<E, A: ErFromTree<E, Input>, Input> ErFromTree<E, Input> for Box<A> {
    fn er_from_tree(tree: ErTree<E>, input: Input) -> Self {
        Self::new(A::er_from_tree(tree, input))
    }

    fn er_from_report(report: ErReport<E>, input: Input) -> Self {
        Self::new(A::er_from_report(report, input))
    }
}
