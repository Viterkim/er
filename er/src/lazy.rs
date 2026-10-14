use crate::ErResult;
use alloc::string::String;

/// A result where the top error has optional string context.
pub type ErLazy<T = ()> = ErResult<T, ErLazyError>;

/// The context added by `.er(())` or `.er(|_| message)`.
#[derive(Debug)]
pub struct ErLazyError(pub Option<String>);
