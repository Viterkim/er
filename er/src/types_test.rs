use crate::ErTree;

/// Return this from a test to print the whole report on failure. When `?` converts an error,
/// it adds `ErFailed` and the location of the `?`. A failure from another ErTest passes through.
pub type ErTest<T = ()> = Result<T, ErFailure>;

/// Return this from `main` to use `?` on errors that fit in the tree. Like `ErTest`, it prints
/// the whole report on failure.
pub type ErMain<T = ()> = Result<T, ErFailure>;

/// A failure from a test or `main`. Prints the whole report and dereferences to its tree.
pub struct ErFailure {
    pub tree: ErTree<ErFailed>,
}

/// The label added where an error enters `ErTest` or `ErMain`.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct ErFailed;

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
