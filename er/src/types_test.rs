use crate::ErReport;

/// Use as a test's return type to print the error report on failure.
pub type TestEr<T = ()> = Result<T, ErReport<TestError>>;

/// Catchall unit struct for tests
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct TestError;
