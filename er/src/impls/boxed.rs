use crate::{
    ErPart, ErReport, ErReportRef, ErSnapshot, ErTop, ErTree, IntoErPart, IntoErSnapshot,
    IntoErTree, IntoErTreeRef,
};
#[cfg(feature = "test")]
use crate::{ErTestError, ErTestFailure};
use alloc::boxed::Box;

macro_rules! forward_part {
    ($($view:ident),+ $(,)?) => {
        $(
            impl<E> IntoErPart for Box<$view<E>>
            where
                $view<E>: IntoErPart,
            {
                type Error = <$view<E> as IntoErPart>::Error;

                fn er_error(&self) -> &Self::Error {
                    <$view<E> as IntoErPart>::er_error(self)
                }

                #[cfg_attr(feature = "src_locations", track_caller)]
                fn into_er_part(self) -> ErPart {
                    <$view<E> as IntoErPart>::into_er_part(*self)
                }
            }
        )+
    };
}

forward_part!(ErTree, ErTop, ErReport);

#[cfg(feature = "test")]
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

impl<E> AsMut<ErTree<E>> for Box<ErReport<E>> {
    fn as_mut(&mut self) -> &mut ErTree<E> {
        &mut self.tree
    }
}
impl<E> AsMut<ErTree<E>> for Box<ErTop<E>> {
    fn as_mut(&mut self) -> &mut ErTree<E> {
        &mut self.tree
    }
}
#[cfg(feature = "test")]
impl AsMut<ErTree<ErTestError>> for Box<ErTestFailure> {
    fn as_mut(&mut self) -> &mut ErTree<ErTestError> {
        &mut self.tree
    }
}

impl<P: IntoErTree> IntoErTree for Box<P> {
    type Error = P::Error;

    fn into_er_tree(self) -> ErTree<Self::Error> {
        (*self).into_er_tree()
    }

    fn into_er_report(self) -> ErReport<Self::Error> {
        (*self).into_er_report()
    }
}
impl<'a, P: IntoErTreeRef<'a>> IntoErTreeRef<'a> for Box<P> {
    type Error = P::Error;

    fn into_er_report_ref(self) -> ErReportRef<'a, Self::Error> {
        (*self).into_er_report_ref()
    }
}
impl<P: IntoErSnapshot> IntoErSnapshot for Box<P> {
    type Top = P::Top;
    type Report = P::Report;

    fn into_er_snapshot(self) -> ErSnapshot {
        (*self).into_er_snapshot()
    }

    fn into_er_top(self) -> Self::Top {
        (*self).into_er_top()
    }

    fn into_er_report(self) -> Self::Report {
        (*self).into_er_report()
    }
}
impl<'a, P: ?Sized> IntoErSnapshot for &'a Box<P>
where
    &'a P: IntoErSnapshot,
{
    type Top = <&'a P as IntoErSnapshot>::Top;
    type Report = <&'a P as IntoErSnapshot>::Report;

    fn into_er_snapshot(self) -> ErSnapshot {
        <&'a P as IntoErSnapshot>::into_er_snapshot(self)
    }

    fn into_er_top(self) -> Self::Top {
        <&'a P as IntoErSnapshot>::into_er_top(self)
    }

    fn into_er_report(self) -> Self::Report {
        <&'a P as IntoErSnapshot>::into_er_report(self)
    }
}
