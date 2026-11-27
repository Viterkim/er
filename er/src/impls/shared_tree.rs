use crate::impls::{
    node::{StoredError, append_nodes},
    tree::with_source,
};
use crate::{
    ErBoxedInput, ErBoxedTreeExt, ErErrorContextExt, ErErrorExt, ErFindAll, ErInput, ErNode,
    ErNodes, ErReportError, ErSavedList, ErSharedTree, ErSharedTreeRefExt, ErSnapshot, ErTree,
    ErWithFields, IntoErNode, IntoErSnapshot,
};
#[cfg(er_unsync)]
use alloc::rc::Rc as Shared;
#[cfg(not(er_unsync))]
use alloc::sync::Arc as Shared;
use alloc::{boxed::Box, string::String, vec::Vec};
use core::{any::TypeId, error::Error, fmt, ops::Deref, ptr};

/// An error that can go in a shared tree.
#[cfg(not(er_unsync))]
#[doc(hidden)]
pub trait ErShareable: Error + Send + Sync + 'static {}
#[cfg(not(er_unsync))]
impl<E: Error + Send + Sync + 'static> ErShareable for E {}

/// An error that can go in a shared tree.
#[cfg(er_unsync)]
#[doc(hidden)]
pub trait ErShareable: Error + 'static {}
#[cfg(er_unsync)]
impl<E: Error + 'static> ErShareable for E {}

trait ErSharedRoot {
    fn view(&self) -> (&StoredError, &[ErNode], &ErSavedList);

    fn into_nodes(self: Shared<Self>) -> Vec<ErNode>;

    fn boxed_address(&self) -> Option<*const ()> {
        None
    }
}
impl<E: ErShareable> ErSharedRoot for ErTree<E> {
    fn view(&self) -> (&StoredError, &[ErNode], &ErSavedList) {
        (&self.top, &self.nodes, &self.saved)
    }

    fn into_nodes(self: Shared<Self>) -> Vec<ErNode> {
        Shared::into_inner(self).map_or_else(Vec::new, tree_nodes)
    }
}

/// The nodes stay a local while the top drops. Returned straight out of the tree they'd already be in the
/// return slot, and a top whose destructor panics would leak every error below it.
fn tree_nodes<E>(tree: ErTree<E>) -> Vec<ErNode> {
    let ErTree {
        top, nodes, saved, ..
    } = tree;

    drop(top);
    drop(saved);

    nodes
}

#[cfg(not(er_unsync))]
type Root = dyn ErSharedRoot + Send + Sync;
#[cfg(er_unsync)]
type Root = dyn ErSharedRoot;

#[cfg(not(er_unsync))]
trait BoxedShareable: ErReportError<ErBoxedInput> + Send + Sync {
    fn stored_error(&self) -> &StoredError;
}
#[cfg(not(er_unsync))]
impl BoxedShareable for crate::ErBoxError {
    fn stored_error(&self) -> &StoredError {
        &**self
    }
}

#[cfg(er_unsync)]
trait BoxedShareable: ErReportError<ErBoxedInput> {
    fn stored_error(&self) -> &StoredError {
        self.as_error()
    }
}
#[cfg(er_unsync)]
impl<E: ErReportError<ErBoxedInput>> BoxedShareable for E {}

// Keep the original shared tree while borrowing the error inside its box.
struct BoxedRoot<E>(Shared<ErTree<E>>);
impl<E: BoxedShareable> ErSharedRoot for BoxedRoot<E> {
    fn view(&self) -> (&StoredError, &[ErNode], &ErSavedList) {
        (self.0.top.stored_error(), &self.0.nodes, &self.0.saved)
    }

    fn into_nodes(self: Shared<Self>) -> Vec<ErNode> {
        Shared::into_inner(self)
            .and_then(|root| Shared::into_inner(root.0))
            .map_or_else(Vec::new, tree_nodes)
    }

    fn boxed_address(&self) -> Option<*const ()> {
        Some(Shared::as_ptr(&self.0).cast())
    }
}

pub(crate) struct ErSharedNode {
    root: Option<Shared<Root>>,
}
impl ErSharedNode {
    pub(crate) fn view(&self) -> Option<(&StoredError, &[ErNode], &ErSavedList)> {
        Some(self.root.as_ref()?.view())
    }

    /// Which shared tree this is, the same for every node holding it.
    pub fn address(&self) -> Option<usize> {
        let root = self.root.as_ref()?;
        let address = root
            .boxed_address()
            .unwrap_or_else(|| Shared::as_ptr(root).cast());

        Some(address.addr())
    }
}
impl Drop for ErSharedNode {
    fn drop(&mut self) {
        let Some(root) = self.root.take() else {
            return;
        };
        let mut pending = root.into_nodes();

        // Take each shared root before its wrapper drops so a long chain uses one stack frame.
        while let Some(mut node) = pending.pop() {
            append_nodes(&mut pending, &mut node.nodes);

            if let Some(shared) = node.error.downcast_mut::<Self>()
                && let Some(root) = shared.root.take()
            {
                append_nodes(&mut pending, &mut root.into_nodes());
            }
        }
    }
}
impl fmt::Display for ErSharedNode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self.view().ok_or(fmt::Error)?.0, formatter)
    }
}
impl fmt::Debug for ErSharedNode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self.view().ok_or(fmt::Error)?.0, formatter)
    }
}
impl Error for ErSharedNode {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.view()?.0.source()
    }
}

impl<E> ErTree<E> {
    /// Freeze the tree so you can clone it and put it below other errors.
    pub fn er_share(self) -> ErSharedTree<E> {
        ErSharedTree {
            tree: Shared::new(self),
        }
    }

    /// Whether this tree is `failure` or has that same shared tree below it.
    pub fn er_contains_shared<T: 'static>(&self, failure: &ErSharedTree<T>) -> bool
    where
        E: 'static,
    {
        let tree = Shared::as_ptr(&failure.tree);
        if TypeId::of::<E>() == TypeId::of::<T>() && ptr::addr_eq(self, tree) {
            return true;
        }

        let boxed = is_boxed_top::<T>();
        ErNodes::once(&self.nodes).any(|node| {
            node.error
                .downcast_ref::<ErSharedNode>()
                .and_then(|shared| shared.root.as_ref())
                .is_some_and(|root| {
                    ptr::addr_eq(Shared::as_ptr(root), tree)
                        || boxed
                            && root
                                .boxed_address()
                                .is_some_and(|address| ptr::addr_eq(address, tree))
                })
        })
    }
}

fn is_boxed_top<T: 'static>() -> bool {
    let ty = TypeId::of::<T>();

    ty == TypeId::of::<crate::ErBoxError>()
        || cfg!(er_unsync)
            && (ty == TypeId::of::<Box<dyn Error + Send + Sync>>()
                || ty == TypeId::of::<Box<dyn Error + Send>>()
                || ty == TypeId::of::<Box<dyn Error + Sync>>())
}

impl<E> ErSharedTree<E> {
    /// Whether both are the same failure, not just equal looking ones.
    pub fn ptr_eq(this: &Self, other: &Self) -> bool {
        Shared::ptr_eq(&this.tree, &other.tree)
    }
}
impl<E: ErShareable> ErSharedTree<E> {
    /// Build the new top from fields taken from the shared top.
    #[track_caller]
    pub fn er_with<A, Mode>(self, fields: impl ErWithFields<E, A, Mode>) -> ErTree<A>
    where
        A: Error + 'static,
    {
        let error = fields.er_with_fields(&self.tree.top);
        with_source(error, self)
    }

    /// Like `.er_with()`, but the closure gets the whole shared tree to search.
    #[track_caller]
    pub fn er_with_tree<A, Mode>(self, fields: impl ErWithFields<Self, A, Mode>) -> ErTree<A>
    where
        A: Error + 'static,
    {
        let error = fields.er_with_fields(&self);
        with_source(error, self)
    }
}

impl<E: ErReportError<ErBoxedInput>> ErSharedTree<E> {
    /// Returns the FIRST match.
    pub fn er_find<T: Error + 'static>(&self) -> Option<&T> {
        ErBoxedTreeExt::er_find(&*self.tree)
    }

    /// Finds all the instances of an error type.
    pub fn er_find_all<T: Error + 'static>(&self) -> ErFindAll<'_, T> {
        ErBoxedTreeExt::er_find_all(&*self.tree)
    }

    /// True if this type is anywhere in the tree or its native sources.
    pub fn er_contains<T: Error + 'static>(&self) -> bool {
        self.er_find::<T>().is_some()
    }
}

impl<E: ErReportError<Input>, Input> IntoErSnapshot<Input> for ErSharedTree<E> {
    fn into_er_snapshot(self) -> ErSnapshot {
        self.tree.er_snapshot()
    }

    fn into_er_report_text(self) -> String {
        self.tree.er_report()
    }
}
impl<E> Clone for ErSharedTree<E> {
    fn clone(&self) -> Self {
        Self {
            tree: Shared::clone(&self.tree),
        }
    }
}
impl<E> Deref for ErSharedTree<E> {
    type Target = ErTree<E>;

    fn deref(&self) -> &ErTree<E> {
        &self.tree
    }
}
impl<E> From<ErTree<E>> for ErSharedTree<E> {
    fn from(tree: ErTree<E>) -> Self {
        tree.er_share()
    }
}
impl<E: ErShareable> IntoErNode for ErSharedTree<E> {
    type Error = E;

    fn er_error(&self) -> &E {
        &self.tree.top
    }

    fn into_er_node(self) -> ErNode {
        shared_node(self.tree, |tree| tree)
    }
}
impl<E: BoxedShareable> IntoErNode<ErBoxedInput> for ErSharedTree<E> {
    type Error = E;

    fn er_error(&self) -> &E {
        &self.tree.top
    }

    fn into_er_node(self) -> ErNode {
        shared_node(self.tree, |tree| Shared::new(BoxedRoot(tree)))
    }
}

fn shared_node<E>(
    tree: Shared<ErTree<E>>,
    erase: impl FnOnce(Shared<ErTree<E>>) -> Shared<Root>,
) -> ErNode {
    ErNode {
        src_location: tree.src_location,
        error: Box::new(ErSharedNode {
            root: Some(erase(tree)),
        }),
        nodes: Vec::new(),
        saved: ErSavedList::default(),
    }
}

impl<E: ErShareable, Mode> ErErrorContextExt<Mode> for ErSharedTree<E> {}
impl<E: ErShareable> IntoErNode for &ErSharedTree<E> {
    type Error = E;

    fn er_error(&self) -> &E {
        &self.tree.top
    }

    fn into_er_node(self) -> ErNode {
        self.clone().into_er_node()
    }
}
impl<E: ErShareable, Mode> ErErrorContextExt<Mode> for &ErSharedTree<E> {}
impl<E: ErShareable> ErErrorExt for &ErSharedTree<E> {}
impl<E: BoxedShareable> IntoErNode<ErBoxedInput> for &ErSharedTree<E> {
    type Error = E;

    fn er_error(&self) -> &E {
        &self.tree.top
    }

    fn into_er_node(self) -> ErNode {
        self.clone().into_er_node()
    }
}
impl<'a, E, Input> IntoErNode<Input> for &'a mut ErSharedTree<E>
where
    &'a ErSharedTree<E>: IntoErNode<Input, Error = E>,
{
    type Error = E;

    fn er_error(&self) -> &E {
        &self.tree.top
    }

    fn into_er_node(self) -> ErNode {
        (&*self).into_er_node()
    }
}
impl<E: BoxedShareable, Mode> ErErrorContextExt<Mode, ErBoxedInput> for ErSharedTree<E> {}
impl<E: BoxedShareable, Mode> ErErrorContextExt<Mode, ErBoxedInput> for &ErSharedTree<E> {}
impl<E: BoxedShareable> ErErrorExt<ErBoxedInput> for ErSharedTree<E> {}
impl<E: BoxedShareable> ErErrorExt<ErBoxedInput> for &ErSharedTree<E> {}
impl<E, Input> ErSharedTreeRefExt<E, Input> for &ErSharedTree<E>
where
    Self: ErInput<Input>,
{
    #[track_caller]
    fn er_with_tree<A, Mode>(self, fields: impl ErWithFields<ErSharedTree<E>, A, Mode>) -> ErTree<A>
    where
        A: Error + 'static,
    {
        with_source(fields.er_with_fields(self), self)
    }
}
impl<E: BoxedShareable> ErSharedTreeRefExt<E, ErBoxedInput> for ErSharedTree<E> {
    #[track_caller]
    fn er_with_tree<A, Mode>(self, fields: impl ErWithFields<Self, A, Mode>) -> ErTree<A>
    where
        A: Error + 'static,
    {
        with_source(fields.er_with_fields(&self), self)
    }
}
