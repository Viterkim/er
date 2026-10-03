use crate::ErTree;

/// Use as a test's return type. `?` adds TestError and its location when converting an error.
/// Another ErTest passes through without adding context. Failures print the whole report.
pub type ErTest<T = ()> = Result<T, ErTestFailure>;

/// A test failure, prints the whole report and borrows like its tree.
pub struct ErTestFailure {
    pub tree: ErTree<ErTestError>,
}

/// The label added when an error enters ErTest.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct ErTestError;

/// The Option was None.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct ErTestOptionNone;
