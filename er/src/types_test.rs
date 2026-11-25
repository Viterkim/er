use crate::ErTree;

/// Return this from a test to print the whole report on failure. When `?` converts an error,
/// it adds `ErTestError` and the location of the `?`. A failure from another ErTest passes through.
pub type ErTest<T = ()> = Result<T, ErTestFailure>;

/// A failed test. Prints the whole report and dereferences to its tree.
pub struct ErTestFailure {
    pub tree: ErTree<ErTestError>,
}

/// The label added where an error enters `ErTest`.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct ErTestError;

/// The Option was None.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct ErTestOptionNone;

/// The test expected an error, but it was Ok.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct ErTestUnexpectedOk;

/// The test got the wrong error. The error it got is below this one.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct ErTestWrongErr {
    pub expected: &'static str,
}
