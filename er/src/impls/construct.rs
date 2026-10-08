use crate::{ErBuilt, ErFields, ErFromTree, ErIntoFields, ErMake, ErReady, ErTree, ErWithFields};

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
impl<E, A, Input, F> ErIntoFields<E, A> for F
where
    F: FnOnce(&ErTree<E>) -> Input,
    A: ErFromTree<E, Input>,
{
    type Input = Input;

    fn er_into_tree(self, tree: ErTree<E>) -> A {
        let values = self(&tree);
        A::er_from_tree(tree, values)
    }
}
