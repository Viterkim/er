#[cfg(feature = "macros")]
use crate::impls::saved::ErSavedError;
#[cfg(feature = "macros")]
use crate::{
    ErAddReady, ErBailNodes, ErBailRoot, ErBailTree, ErBranch, ErNew, ErNewFields, ErNewPass,
    ErPrepared, ErReadyTree, ErReportError, ErTree, IntoErTree,
};
use crate::{
    ErBuildFrom, ErDefaults, ErErrorWithSaved, ErFieldInput, ErFields, ErFieldsFor, ErForm,
    ErFromFields, ErMake, ErNode, ErReady, ErValFields, ErValueForm, ErWhole, ErWithFields,
    ErWithSaves,
};
use alloc::vec::Vec;
use core::error::Error;
use core::fmt;
#[cfg(feature = "macros")]
use core::marker::PhantomData;

#[cfg(feature = "macros")]
impl<T, E> ErBailTree<T> for ErTree<E>
where
    Self: Into<T>,
{
    #[track_caller]
    fn finish(self, _: PhantomData<T>) -> T {
        self.into()
    }
}
#[cfg(feature = "macros")]
impl<P: ErReadyTree, T> ErBailNodes<ErBailRoot<P, ErReady>, T, ErReady> for u8
where
    ErTree<P::Top>: Into<T>,
{
    type Top = P::Top;

    #[track_caller]
    fn prepare(self, root: ErBailRoot<P, ErReady>) -> (PhantomData<T>, ErTree<Self::Top>) {
        let tree = root.0.into_ready_tree();
        (PhantomData, tree)
    }
}
#[cfg(feature = "macros")]
impl<T, P, E, Form> ErBailNodes<ErBailRoot<P, ErFields>, T, (E, Form)> for u8
where
    ErTree<E>: Into<T>,
    ErNew<P>: ErNewFields<E, Form>,
{
    type Top = E;

    #[track_caller]
    fn prepare(self, root: ErBailRoot<P, ErFields>) -> (PhantomData<T>, ErTree<E>) {
        let tree = ErNewFields::__er_finish(ErNew(root.0));
        (PhantomData, tree)
    }
}
#[cfg(feature = "macros")]
impl<P> ErNewPass<P> for ErNew<P> {
    fn __er_bail_value(self) -> P {
        self.0
    }

    fn __er_bail_root(self) -> ErBailRoot<P, ErFields> {
        ErBailRoot(self.0, PhantomData)
    }
}

#[cfg(feature = "macros")]
impl<E> ErNew<ErBranch<E>> {
    pub fn __er_add_pending(self) -> ErPrepared<ErBranch<E>> {
        ErPrepared(self.0)
    }

    pub fn __er_bail_value(self) -> ErPrepared<ErBranch<E>> {
        ErPrepared(self.0)
    }
}
#[cfg(feature = "macros")]
impl<E> ErNew<ErErrorWithSaved<E>> {
    pub fn __er_add_pending<Input>(self) -> ErPrepared<E::Root>
    where
        E: ErSavedError<Input>,
    {
        ErPrepared(self.0.value.with_saved(self.0.saved))
    }
}
#[cfg(feature = "macros")]
impl<T> ErPrepared<T> {
    pub fn __er_tree(self) -> Self {
        self
    }

    pub fn __er_add_ready(self) -> T {
        self.0
    }
}
#[cfg(feature = "macros")]
impl<E> ErNew<ErTree<E>> {
    pub fn __er_add_ready(self) -> ErTree<E> {
        self.0
    }
}
#[cfg(feature = "macros")]
impl<E: ErReportError<Input>, Input> ErAddReady<Input> for ErNew<E> {
    type Error = E;

    fn __er_add_ready(self) -> ErBranch<E> {
        ErBranch::from_error(self.0)
    }
}
#[cfg(feature = "macros")]
impl<T: IntoErTree> ErNew<T> {
    #[track_caller]
    pub fn __er_tree(self) -> ErNew<ErTree<T::Error>> {
        ErNew(self.0.into_er_tree())
    }
}
#[cfg(feature = "macros")]
impl<F: FnOnce() -> E, E> ErNew<F> {
    #[track_caller]
    pub fn __er_factory(self) -> ErNew<E> {
        ErNew((self.0)())
    }
}
#[cfg(feature = "macros")]
impl<P: ErReadyTree> ErNew<P> {
    #[track_caller]
    pub fn __er_finish(self) -> ErTree<P::Top> {
        self.0.into_ready_tree()
    }

    pub fn __er_bail_root(self) -> ErBailRoot<P, ErReady> {
        ErBailRoot(self.0, PhantomData)
    }
}
#[cfg(feature = "macros")]
impl<E: Error + 'static> ErReadyTree for E {
    type Top = E;

    #[track_caller]
    fn into_ready_tree(self) -> ErTree<E> {
        ErTree::from(self)
    }
}
#[cfg(feature = "macros")]
impl<E: Error + 'static, Nodes: Into<Vec<ErNode>>> ErReadyTree for ErErrorWithSaved<E, Nodes> {
    type Top = E;

    #[track_caller]
    fn into_ready_tree(self) -> ErTree<E> {
        self.into_tree()
    }
}
#[cfg(feature = "macros")]
impl<E> ErReadyTree for ErTree<E> {
    type Top = E;

    fn into_ready_tree(self) -> ErTree<E> {
        self
    }
}
#[cfg(feature = "macros")]
impl<E, P, Form: ErValueForm> ErNewFields<E, Form> for ErNew<P>
where
    E: Error + ErFromFields<P, Form> + 'static,
{
    #[track_caller]
    fn __er_finish(self) -> ErTree<E> {
        ErTree::from(E::er_from_fields(self.0))
    }
}
#[cfg(feature = "macros")]
impl<E, F, P, Mode> ErNewFields<E, (ErReady, Mode)> for ErNew<F>
where
    F: FnOnce(()) -> P + ErMake<E, Mode>,
    E: Error + 'static,
{
    #[track_caller]
    fn __er_finish(self) -> ErTree<E> {
        self.0.er_make().into_tree()
    }
}
#[cfg(feature = "macros")]
impl<E, P, Nodes, Form: ErValueForm> ErNewFields<E, (ErWithSaves, Form)>
    for ErNew<ErErrorWithSaved<P, Nodes>>
where
    Nodes: Into<Vec<ErNode>>,
    E: Error + ErFromFields<P, Form> + 'static,
{
    #[track_caller]
    fn __er_finish(self) -> ErTree<E> {
        self.0.build::<E, Form>().into_tree()
    }
}

impl ErForm for ErFields {}
impl<const GIVEN: usize> ErForm for ErDefaults<GIVEN> {}
impl ErForm for ErWhole {}
impl ErValueForm for ErFields {}
impl<const GIVEN: usize> ErValueForm for ErDefaults<GIVEN> {}

impl<A, P, Form: ErValueForm> ErBuildFrom<P, Form> for A
where
    A: ErFromFields<P, Form>,
{
    #[track_caller]
    fn er_build_from(fields: P) -> A {
        A::er_from_fields(fields)
    }
}
impl<A> ErBuildFrom<A, ErWhole> for A {
    fn er_build_from(error: A) -> A {
        error
    }
}

impl<A: Error, F: FnOnce() -> A> ErMake<A, ErReady> for F {
    type Nodes = [ErNode; 0];

    const COLD: bool = core::mem::needs_drop::<A>();
    #[track_caller]
    fn er_make(self) -> ErErrorWithSaved<A, Self::Nodes> {
        ErErrorWithSaved::new(self())
    }
}
impl<A, Nodes, F: FnOnce() -> ErErrorWithSaved<A, Nodes>> ErMake<A, (ErReady, ErWithSaves)> for F
where
    Nodes: Into<Vec<ErNode>>,
{
    type Nodes = Nodes;

    #[track_caller]
    fn er_make(self) -> ErErrorWithSaved<A, Self::Nodes> {
        self()
    }
}

impl<A, P, F, Form: ErForm> ErMake<A, Form> for F
where
    F: FnOnce(()) -> P,
    ErFieldInput<P>: ErFieldsFor<A, Form>,
    A: ErBuildFrom<P, Form>,
{
    type Nodes = [ErNode; 0];

    const COLD: bool = core::mem::needs_drop::<A>();

    #[track_caller]
    fn er_make(self) -> ErErrorWithSaved<A, Self::Nodes> {
        ErErrorWithSaved::new(A::er_build_from(self(())))
    }
}
impl<A, P, Nodes, F, Form: ErForm> ErMake<A, (ErWithSaves, Form)> for F
where
    F: FnOnce(()) -> ErErrorWithSaved<P, Nodes>,
    Nodes: Into<Vec<ErNode>>,
    ErFieldInput<P>: ErFieldsFor<A, Form>,
    A: ErBuildFrom<P, Form>,
{
    type Nodes = Nodes;

    #[track_caller]
    fn er_make(self) -> ErErrorWithSaved<A, Self::Nodes> {
        self(()).build()
    }
}

// When the fields don't fit, the message about ErMake says what to do, the one about a missing `Copy` deep inside
// the tuple didn't.
#[diagnostic::do_not_recommend]
impl<A, P: Copy + fmt::Debug, Form: ErValueForm> ErMake<A, (Form,)> for P
where
    A: ErFromFields<P, Form>,
{
    type Nodes = [ErNode; 0];

    // Plain data is cheap to build in place. A String or PathBuf conversion isn't.
    const COLD: bool = core::mem::needs_drop::<A>();

    #[track_caller]
    fn er_make(self) -> ErErrorWithSaved<A, Self::Nodes> {
        ErErrorWithSaved::new(A::er_from_fields(self))
    }
}

impl<E: ?Sized, A, Input, F, Form: ErForm> ErWithFields<E, A, Form> for F
where
    F: FnOnce(&E) -> Input,
    ErFieldInput<Input>: ErFieldsFor<A, Form>,
    A: ErBuildFrom<Input, Form>,
{
    type Nodes = [ErNode; 0];
    type Input = Input;

    #[track_caller]
    fn er_with_fields(self, error: &E) -> ErErrorWithSaved<A, Self::Nodes> {
        ErErrorWithSaved::new(A::er_build_from(self(error)))
    }
}
impl<E: ?Sized, A, P, Nodes, F, Form: ErForm> ErWithFields<E, A, (ErWithSaves, Form)> for F
where
    F: FnOnce(&E) -> ErErrorWithSaved<P, Nodes>,
    Nodes: Into<Vec<ErNode>>,
    ErFieldInput<P>: ErFieldsFor<A, Form>,
    A: ErBuildFrom<P, Form>,
{
    type Nodes = Nodes;
    type Input = ErErrorWithSaved<P, Nodes>;

    #[track_caller]
    fn er_with_fields(self, error: &E) -> ErErrorWithSaved<A, Self::Nodes> {
        self(error).build()
    }
}
impl<E, A, Input, F, Form: ErForm> ErValFields<E, A, Form> for F
where
    F: FnOnce(E) -> Input,
    ErFieldInput<Input>: ErFieldsFor<A, Form>,
    A: ErBuildFrom<Input, Form>,
{
    type Nodes = [ErNode; 0];
    type Input = Input;

    #[track_caller]
    fn er_val_fields(self, value: E) -> ErErrorWithSaved<A, Self::Nodes> {
        ErErrorWithSaved::new(A::er_build_from(self(value)))
    }
}
impl<E, A, P, Nodes, F, Form: ErForm> ErValFields<E, A, (ErWithSaves, Form)> for F
where
    F: FnOnce(E) -> ErErrorWithSaved<P, Nodes>,
    Nodes: Into<Vec<ErNode>>,
    ErFieldInput<P>: ErFieldsFor<A, Form>,
    A: ErBuildFrom<P, Form>,
{
    type Nodes = Nodes;
    type Input = ErErrorWithSaved<P, Nodes>;

    #[track_caller]
    fn er_val_fields(self, value: E) -> ErErrorWithSaved<A, Self::Nodes> {
        self(value).build()
    }
}
