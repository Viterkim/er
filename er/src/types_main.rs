use crate::ErTree;

/// Return this from `main` to print the whole report when it fails. Use `.er(..)?` like in any other function.
/// With `ErMain<MyErr, ExitCode>` your program can pick its own exit status on success.
pub type ErMain<E = ErMainError, T = ()> = Result<T, ErMainFailure<E>>;

/// A failed `main`. Prints the whole report on exit.
pub struct ErMainFailure<E = ErMainError> {
    pub tree: ErTree<E>,
}

/// What `.er(())` builds in a `main` returning plain `ErMain`.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct ErMainError;
