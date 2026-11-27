#[cfg(feature = "macros")]
use crate::{ErAggregateAdd, ErMake};
use crate::{ErAllItem, ErBranch, ErErrorWithSaved, ErInput, ErNode, ErTree, IntoErTree};
use alloc::{
    boxed::Box,
    collections::{BTreeMap, BTreeSet, BinaryHeap, LinkedList, VecDeque},
    vec::Vec,
};
#[cfg(feature = "macros")]
use core::mem;
use core::{
    error::Error,
    iter::{Once, once},
};

#[doc(hidden)]
pub trait ErAddRoot: IntoErTree {
    fn child_nodes(&mut self) -> &mut Vec<ErNode>;
}
impl<E> ErAddRoot for ErBranch<E> {
    fn child_nodes(&mut self) -> &mut Vec<ErNode> {
        &mut self.nodes
    }
}
impl<E> ErAddRoot for ErTree<E> {
    fn child_nodes(&mut self) -> &mut Vec<ErNode> {
        &mut self.nodes
    }
}

// Batch macros evaluate the whole top expression on failure, so even an owned field is lazy.
#[cfg(feature = "macros")]
#[doc(hidden)]
pub trait ErBatchTop<A, Mode> {
    type Nodes: Into<Vec<ErNode>>;

    #[track_caller]
    fn into_batch_top(self) -> ErErrorWithSaved<A, Self::Nodes>;
}
#[cfg(feature = "macros")]
impl<A, P: core::fmt::Debug, Form: crate::ErForm> ErBatchTop<A, (Form,)> for P
where
    crate::ErFieldInput<P>: crate::ErFieldsFor<A, Form>,
    A: crate::ErBuildFrom<P, Form>,
{
    type Nodes = [ErNode; 0];

    fn into_batch_top(self) -> ErErrorWithSaved<A, Self::Nodes> {
        ErErrorWithSaved::new(A::er_build_from(self))
    }
}
#[cfg(feature = "macros")]
impl<A, P, Nodes, Form: crate::ErForm> ErBatchTop<A, (crate::ErWithSaves, Form)>
    for ErErrorWithSaved<P, Nodes>
where
    Nodes: Into<Vec<ErNode>>,
    crate::ErFieldInput<P>: crate::ErFieldsFor<A, Form>,
    A: crate::ErBuildFrom<P, Form>,
{
    type Nodes = Nodes;

    fn into_batch_top(self) -> ErErrorWithSaved<A, Self::Nodes> {
        self.build()
    }
}
#[cfg(feature = "macros")]
impl<A, F: FnOnce() -> P + ErMake<A, Mode>, P, Mode> ErBatchTop<A, (crate::ErReady, Mode)> for F {
    type Nodes = F::Nodes;

    fn into_batch_top(self) -> ErErrorWithSaved<A, Self::Nodes> {
        self.er_make()
    }
}
#[cfg(feature = "macros")]
impl<A, F: FnOnce(()) -> P + ErMake<A, Mode>, P, Mode> ErBatchTop<A, (crate::ErFields, Mode)>
    for F
{
    type Nodes = F::Nodes;

    fn into_batch_top(self) -> ErErrorWithSaved<A, Self::Nodes> {
        self.er_make()
    }
}

#[diagnostic::on_unimplemented(
    message = "can't collect errors from `{Self}`",
    label = "pass an error, a Result, an iterator or a collection like Vec",
    note = "for other collections, pass their iterator with `.into_iter()`"
)]
#[doc(hidden)]
pub trait ErAggregateItems<Kind> {
    type Item;
    type Iter: Iterator<Item = Self::Item>;

    fn er_items(self) -> Self::Iter;
}

pub struct ErItemsSingle;
impl<I: ErInput<Input>, Input> ErAggregateItems<(ErItemsSingle, Input)> for I {
    type Item = I;
    type Iter = Once<I>;

    fn er_items(self) -> Self::Iter {
        once(self)
    }
}

pub struct ErItemsIter;
impl<I: Iterator> ErAggregateItems<ErItemsIter> for I {
    type Item = I::Item;
    type Iter = I;

    fn er_items(self) -> Self::Iter {
        self
    }
}
impl<T, E> ErAggregateItems<()> for Result<T, E> {
    type Item = Self;
    type Iter = Once<Self>;

    fn er_items(self) -> Self::Iter {
        once(self)
    }
}
impl<'a, T, E> ErAggregateItems<()> for &'a Result<T, E> {
    type Item = Result<&'a T, &'a E>;
    type Iter = Once<Self::Item>;

    fn er_items(self) -> Self::Iter {
        once(self.as_ref())
    }
}
impl<'a, T, E> ErAggregateItems<()> for &'a mut Result<T, E> {
    type Item = Result<&'a T, &'a E>;
    type Iter = Once<Self::Item>;

    fn er_items(self) -> Self::Iter {
        (&*self).er_items()
    }
}

macro_rules! collections {
    ($(($($generic:tt)*) $ty:ty),* $(,)?) => {$(
        collections!(@impl ($($generic)*) $ty);
        collections!(@impl ('a, $($generic)*) &'a $ty);
        collections!(@impl ('a, $($generic)*) &'a mut $ty);
    )*};
    (@impl ($($generic:tt)*) $ty:ty) => {
        impl<$($generic)*> ErAggregateItems<()> for $ty
        where
            Self: IntoIterator,
        {
            type Item = <Self as IntoIterator>::Item;
            type Iter = <Self as IntoIterator>::IntoIter;

            fn er_items(self) -> Self::Iter {
                self.into_iter()
            }
        }
    };
}

// A blanket IntoIterator impl would make a bare Result feed us its Ok value.
collections!(
    (T) Option<T>,
    (T, const N: usize) [T; N],
    (T) Box<[T]>,
    (T) Vec<T>,
    (T) VecDeque<T>,
    (T) LinkedList<T>,
    (T) BinaryHeap<T>,
    (T) BTreeSet<T>,
    (K, V) BTreeMap<K, V>,
);
collections!(@impl ('a, T) &'a [T]);
collections!(@impl ('a, T) &'a mut [T]);

#[doc(hidden)]
#[track_caller]
pub fn tree_from_nodes<A: Error + 'static>(
    mut nodes: Vec<ErNode>,
    top: ErErrorWithSaved<A, impl Into<Vec<ErNode>>>,
) -> ErTree<A> {
    let mut top = top.into_branch();
    crate::impls::node::append_nodes(&mut top.nodes, &mut nodes);
    top.into()
}

/// `er_all!` over an iterable. An iterator of bare errors has no Ok to skip, so its size hint is the node count.
#[cfg(feature = "macros")]
#[doc(hidden)]
#[track_caller]
pub fn gather_items<I, Tag, Mode>(tag: Tag, results: I) -> Vec<ErNode>
where
    I: Iterator,
    Tag: crate::internal::ErAggregateItem<I::Item, Mode> + Copy,
{
    let mut nodes: Vec<ErNode> = Vec::new();

    if Tag::ALWAYS_ERROR {
        nodes.reserve_exact(results.size_hint().0);
    }

    for result in results {
        if let Err(node) = tag.item(result) {
            push_node(&mut nodes, node);
        }
    }

    nodes
}

// Inlining the capacity check lets rustc write the node straight into its Vec slot.
#[inline(always)]
fn push_node(nodes: &mut Vec<ErNode>, node: ErNode) {
    if nodes.len() == nodes.capacity() {
        push_growing(nodes, node);
    } else {
        nodes.push(node);
    }
}

#[cold]
#[inline(never)]
fn push_growing(nodes: &mut Vec<ErNode>, node: ErNode) {
    if nodes.is_empty() {
        *nodes = alloc::vec![node];
    } else {
        nodes.push(node);
    }
}

#[cfg(feature = "macros")]
#[doc(hidden)]
pub fn gather_nodes(items: &mut [Result<(), ErNode>]) -> Vec<ErNode> {
    let mut nodes = Vec::new();
    take_failures(&mut nodes, items);
    nodes
}

#[cfg(feature = "macros")]
#[inline(never)]
fn take_failures(nodes: &mut Vec<ErNode>, items: &mut [Result<(), ErNode>]) {
    let mut items = items.iter_mut();
    while let Some(item) = items.find(|item| item.is_err()) {
        let Err(node) = mem::replace(item, Ok(())) else {
            continue;
        };

        if nodes.len() == nodes.capacity() {
            reserve_nodes(nodes, items.as_slice());
        }

        nodes.push(node);
    }
}

#[cfg(feature = "macros")]
#[cold]
#[inline(never)]
fn reserve_nodes(nodes: &mut Vec<ErNode>, remaining: &[Result<(), ErNode>]) {
    let count = 1 + remaining.iter().filter(|item| item.is_err()).count();
    if nodes.is_empty() {
        nodes.reserve_exact(count);
    } else {
        nodes.reserve(count);
    }
}

#[cfg(feature = "macros")]
#[doc(hidden)]
#[inline]
#[track_caller]
pub fn add_one<R, T, Mode>(mut root: R, tag: impl ErAggregateAdd<Once<T>, Mode>, item: T) -> R
where
    R: ErAddRoot,
{
    tag.add(root.child_nodes(), once(item));
    root
}

#[cfg(feature = "macros")]
#[doc(hidden)]
#[inline]
pub fn add_nodes<R: ErAddRoot>(mut root: R, items: &mut [Result<(), ErNode>]) -> R {
    take_failures(root.child_nodes(), items);
    root
}

#[track_caller]
pub fn append<R: ErAllItem<Mode>, Mode, Kind>(
    nodes: &mut Vec<ErNode>,
    results: impl ErAggregateItems<Kind, Item = R>,
) {
    let results = results.er_items();
    if R::ALWAYS_ERROR {
        let count = results.size_hint().0;
        if nodes.capacity() == 0 {
            nodes.reserve_exact(count);
        } else {
            nodes.reserve(count);
        }
    }

    for result in results {
        if let Err(node) = result.er_all_item() {
            push_node(nodes, node);
        }
    }
}

#[cfg(feature = "macros")]
#[doc(hidden)]
#[track_caller]
pub fn try_all_1<A>(result: Result<A, ErNode>) -> Result<(A,), Vec<ErNode>> {
    match result {
        Ok(value) => Ok((value,)),
        Err(node) => Err(alloc::vec![node]),
    }
}

macro_rules! try_all {
    ($($name:ident: $($value:ident $result:ident),+;)+) => {$(
        #[cfg(feature = "macros")]
        #[doc(hidden)]
        #[track_caller]
        #[inline]
        #[allow(clippy::too_many_arguments)]
        pub fn $name<$($value),+>(
            $($result: Result<$value, ErNode>),+
        ) -> Result<($($value,)+), Vec<ErNode>> {
            match ($($result,)+) {
                ($(Ok($result),)+) => Ok(($($result,)+)),
                ($($result,)+) => Err(gather_nodes(&mut [$($result.map(drop)),+])),
            }
        }
    )+};
}

try_all! {
    try_all_2: A a, B b;
    try_all_3: A a, B b, C c;
    try_all_4: A a, B b, C c, D d;
    try_all_5: A a, B b, C c, D d, E e;
    try_all_6: A a, B b, C c, D d, E e, F f;
    try_all_7: A a, B b, C c, D d, E e, F f, G g;
    try_all_8: A a, B b, C c, D d, E e, F f, G g, H h;
    try_all_9: A a, B b, C c, D d, E e, F f, G g, H h, I i;
    try_all_10: A a, B b, C c, D d, E e, F f, G g, H h, I i, J j;
    try_all_11: A a, B b, C c, D d, E e, F f, G g, H h, I i, J j, K k;
    try_all_12: A a, B b, C c, D d, E e, F f, G g, H h, I i, J j, K k, L l;
}
