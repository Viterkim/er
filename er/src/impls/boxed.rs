use crate::{
    ErOpaqueErrorExt, ErReport, ErReportRef, ErSnapshot, ErTop, ErTree, IntoErSnapshot, IntoErTree,
    IntoErTreeRef,
};
#[cfg(feature = "test")]
use crate::{ErTestError, ErTestFailure};
use alloc::boxed::Box;

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
impl<P: ErOpaqueErrorExt> ErOpaqueErrorExt for Box<P> {
    type Output = P::Output;

    fn opaque_err(self) -> Self::Output {
        (*self).opaque_err()
    }
}
impl<'a, P: ?Sized> ErOpaqueErrorExt for &'a Box<P>
where
    &'a P: ErOpaqueErrorExt,
{
    type Output = <&'a P as ErOpaqueErrorExt>::Output;

    fn opaque_err(self) -> Self::Output {
        <&'a P as ErOpaqueErrorExt>::opaque_err(self)
    }
}
