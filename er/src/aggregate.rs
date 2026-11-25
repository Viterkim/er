use crate::{
    ErAllItem, ErErrorWithSaved, ErInput, ErMake, ErNode, ErPart, ErResult, ErSavedList, ErTree,
};
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

// Batch macros evaluate the whole top expression on failure, so even an owned field is lazy.
#[cfg(feature = "macros")]
#[doc(hidden)]
pub trait ErBatchTop<A, Mode> {
    #[track_caller]
    fn into_batch_top(self) -> ErErrorWithSaved<A>;
}
#[cfg(feature = "macros")]
impl<A, P: core::fmt::Debug, Form: crate::ErForm> ErBatchTop<A, (Form,)> for P
where
    crate::ErFieldInput<P>: crate::ErFieldsFor<A, Form>,
    A: crate::ErBuildFrom<P, Form>,
{
    fn into_batch_top(self) -> ErErrorWithSaved<A> {
        ErErrorWithSaved::new(A::er_build_from(self))
    }
}
#[cfg(feature = "macros")]
impl<A, P, Form: crate::ErForm> ErBatchTop<A, (crate::ErWithSaves, Form)> for ErErrorWithSaved<P>
where
    crate::ErFieldInput<P>: crate::ErFieldsFor<A, Form>,
    A: crate::ErBuildFrom<P, Form>,
{
    fn into_batch_top(self) -> ErErrorWithSaved<A> {
        self.build()
    }
}
#[cfg(feature = "macros")]
impl<A, F: FnOnce() -> P + ErMake<A, Mode>, P, Mode> ErBatchTop<A, (crate::ErReady, Mode)> for F {
    fn into_batch_top(self) -> ErErrorWithSaved<A> {
        self.er_make()
    }
}
#[cfg(feature = "macros")]
impl<A, F: FnOnce(()) -> P + ErMake<A, Mode>, P, Mode> ErBatchTop<A, (crate::ErFields, Mode)>
    for F
{
    fn into_batch_top(self) -> ErErrorWithSaved<A> {
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
pub struct CollectedErrors {
    pub nodes: Vec<ErNode>,
    pub saved: ErSavedList,
}
impl CollectedErrors {
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    #[track_caller]
    pub fn into_tree<A: Error + 'static>(mut self, top: ErErrorWithSaved<A>) -> ErTree<A> {
        let ErErrorWithSaved { value, mut saved } = top;
        self.saved.append(&mut saved);

        ErTree {
            nodes: self.nodes,
            saved: self.saved,
            ..ErTree::from(value)
        }
    }

    #[track_caller]
    pub fn finish<A, Mode>(self, top: impl ErMake<A, Mode>) -> ErResult<(), A>
    where
        A: Error + 'static,
    {
        if self.is_empty() {
            Ok(())
        } else {
            Err(self.into_tree(top.er_make()))
        }
    }
}

#[doc(hidden)]
#[track_caller]
pub fn gather<E, T, Input, Kind>(
    results: impl ErAggregateItems<Kind, Item = Result<T, E>>,
) -> CollectedErrors
where
    E: ErInput<Input>,
{
    let mut nodes: Vec<ErNode> = Vec::new();
    let mut saved = ErSavedList::default();

    for result in results.er_items() {
        match result {
            Ok(value) => drop(value),
            Err(error) => {
                let ErPart {
                    node,
                    saved: mut incoming,
                } = error.into_er_input();
                saved.append(&mut incoming);

                if nodes.len() == nodes.capacity() {
                    push_growing(&mut nodes, node);
                } else {
                    nodes.push(node);
                }
            }
        }
    }

    CollectedErrors { nodes, saved }
}

/// `er_all!` over an iterable. An iterator of bare errors has no Ok to skip, so its size hint is the node count.
#[cfg(feature = "macros")]
#[doc(hidden)]
#[track_caller]
pub fn gather_items<I, Tag, Mode>(tag: Tag, results: I) -> CollectedErrors
where
    I: Iterator,
    Tag: crate::internal::ErAggregateItem<I::Item, Mode> + Copy,
{
    let mut nodes: Vec<ErNode> = Vec::new();
    let mut saved = ErSavedList::default();

    if Tag::ALWAYS_ERROR {
        nodes.reserve_exact(results.size_hint().0);
    }

    for result in results {
        match tag.item(result) {
            Ok(value) => drop(value),
            Err(ErPart {
                node,
                saved: mut incoming,
            }) => {
                saved.append(&mut incoming);

                if nodes.len() == nodes.capacity() {
                    push_growing(&mut nodes, node);
                } else {
                    nodes.push(node);
                }
            }
        }
    }

    CollectedErrors { nodes, saved }
}

// Moving the node into this call keeps a stack copy out of the loop when there's room.
#[cold]
#[inline(never)]
fn push_growing(nodes: &mut Vec<ErNode>, node: ErNode) {
    if nodes.is_empty() {
        nodes.reserve_exact(1);
    }
    nodes.push(node);
}

/// Keep every failure in `results`. Build the top error only if a result failed.
#[track_caller]
pub fn collect<A, E, T, Input, Mode, Kind>(
    top: impl ErMake<A, Mode>,
    results: impl ErAggregateItems<Kind, Item = Result<T, E>>,
) -> ErResult<(), A>
where
    A: Error + 'static,
    E: ErInput<Input>,
{
    gather(results).finish(top)
}

#[cfg(feature = "macros")]
#[doc(hidden)]
pub fn gather_parts(items: &mut [Result<(), ErPart>]) -> CollectedErrors {
    let mut errors = CollectedErrors {
        nodes: Vec::new(),
        saved: ErSavedList::default(),
    };
    append_parts(&mut errors.nodes, &mut errors.saved, items);
    errors
}

#[cfg(feature = "macros")]
#[inline(never)]
fn append_parts(
    nodes: &mut Vec<ErNode>,
    reports: &mut ErSavedList,
    items: &mut [Result<(), ErPart>],
) {
    let mut items = items.iter_mut();
    while let Some(item) = items.find(|item| item.is_err()) {
        let Err(ErPart { node, mut saved }) = mem::replace(item, Ok(())) else {
            continue;
        };

        if nodes.len() == nodes.capacity() {
            reserve_parts(nodes, items.as_slice());
        }

        reports.append(&mut saved);
        nodes.push(node);
    }
}

#[cfg(feature = "macros")]
#[cold]
#[inline(never)]
fn reserve_parts(nodes: &mut Vec<ErNode>, remaining: &[Result<(), ErPart>]) {
    let count = 1 + remaining.iter().filter(|item| item.is_err()).count();
    if nodes.is_empty() {
        nodes.reserve_exact(count);
    } else {
        nodes.reserve(count);
    }
}

#[cfg(feature = "macros")]
#[doc(hidden)]
#[track_caller]
pub fn add_parts<T: crate::IntoErTree>(
    tree: T,
    items: &mut [Result<(), ErPart>],
) -> ErTree<T::Error> {
    let mut tree = tree.into_er_tree();
    append_parts(&mut tree.nodes, &mut tree.saved, items);
    tree
}

#[track_caller]
pub fn append<R: ErAllItem<Mode>, Mode, Kind>(
    nodes: &mut Vec<ErNode>,
    saved: &mut ErSavedList,
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
        let Err(part) = result.er_all_item() else {
            continue;
        };

        // Borrowing part.saved here made rustc copy the whole node again before pushing it.
        let ErPart {
            node,
            saved: mut incoming,
        } = part;
        saved.append(&mut incoming);
        if nodes.len() == nodes.capacity() {
            push_growing(nodes, node);
        } else {
            nodes.push(node);
        }
    }
}

#[cfg(feature = "macros")]
#[doc(hidden)]
#[track_caller]
#[allow(clippy::result_large_err)]
pub fn try_all_1<A>(result: Result<A, ErPart>) -> Result<(A,), CollectedErrors> {
    match result {
        Ok(value) => Ok((value,)),
        Err(part) => Err(CollectedErrors {
            nodes: alloc::vec![part.node],
            saved: part.saved,
        }),
    }
}

macro_rules! try_all {
    ($($name:ident: $($value:ident $result:ident),+;)+) => {$(
        #[cfg(feature = "macros")]
        #[doc(hidden)]
        #[track_caller]
        #[allow(clippy::result_large_err, clippy::too_many_arguments)]
        pub fn $name<$($value),+>(
            $($result: Result<$value, ErPart>),+
        ) -> Result<($($value,)+), CollectedErrors> {
            match ($($result,)+) {
                ($(Ok($result),)+) => Ok(($($result,)+)),
                ($($result,)+) => Err(gather_parts(&mut [$($result.map(drop)),+])),
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
