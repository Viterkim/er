use crate::{ErPart, ErTree, IntoErPart, IntoErTree, IntoErTreeRef};
use crate::{ErTestError, ErTestFailure};
use alloc::boxed::Box;

impl<E, Input> IntoErPart<Input> for Box<ErTree<E>>
where
    ErTree<E>: IntoErPart<Input>,
{
    type Error = <ErTree<E> as IntoErPart<Input>>::Error;

    fn er_error(&self) -> &Self::Error {
        <ErTree<E> as IntoErPart<Input>>::er_error(self)
    }

    #[cfg_attr(feature = "src_locations", track_caller)]
    fn into_er_part(self) -> ErPart {
        <ErTree<E> as IntoErPart<Input>>::into_er_part(*self)
    }
}

impl IntoErPart for Box<ErTestFailure> {
    type Error = ErTestError;

    fn er_error(&self) -> &ErTestError {
        &self.tree.top
    }

    #[cfg_attr(feature = "src_locations", track_caller)]
    fn into_er_part(self) -> ErPart {
        self.tree.into_er_part()
    }
}

impl<P: IntoErTree> IntoErTree for Box<P> {
    type Error = P::Error;

    fn into_er_tree(self) -> ErTree<Self::Error> {
        (*self).into_er_tree()
    }
}
impl<E> From<Box<ErTree<E>>> for ErTree<E> {
    fn from(tree: Box<ErTree<E>>) -> Self {
        tree.into_er_tree()
    }
}
impl<'a, P: IntoErTreeRef<'a>> IntoErTreeRef<'a> for Box<P> {
    type Error = P::Error;

    fn into_er_tree_ref(self) -> &'a ErTree<Self::Error> {
        (*self).into_er_tree_ref()
    }
}
