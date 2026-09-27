/// Run every result and return their values as a tuple if all succeed.
/// If anything fails, keep every error and drop the successful values.
/// The top error only gets made on failure. Results are evaluated in list order.
/// Bare errors and trees are failures with `()` as their success slot.
///
/// `let (port, enabled) = er_try!((), [read_port(port), read_enabled(enabled)])?;`
/// For an iterator of matching types, use `.er_collect_all()`.
#[macro_export]
macro_rules! er_try {
    ($top:expr, [] $(,)?) => {{
        $crate::er_all!($top, [])
    }};
    ($top:expr, [$($result:expr),+ $(,)?] $(,)?) => {{
        $crate::__er_try_results!($crate, $top, [$($result),*])
    }};
}
