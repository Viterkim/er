use crate::ErTree;

/// Use as a test's return type. `?` adds TestError and its location when converting an error.
/// Another ErTest passes through without adding context. Failures print the whole report.
pub type ErTest<T = ()> = Result<T, ErTestFailure>;

/// A test failure and its original errors. Access them through `.tree`.
/// No Error or IntoErPart impl, so the catchall From impl doesn't overlap `From<Self>`.
pub struct ErTestFailure {
    pub tree: ErTree<ErTestError>,
}

/// The label added when an error enters ErTest.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct ErTestError;
