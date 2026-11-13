/// Run every result and return their values as a tuple if all succeed.
/// If anything fails, keep every error and drop the successful values.
/// Evaluate results in list order. A constructor in the first argument runs only if something fails.
/// Bare errors and trees are failures with `()` as their success slot.
/// An optional check gives you its successful value as `Some`, or `None` when the check was absent.
///
/// `let (port, enabled) = er_try!((), [read_port(port), read_enabled(enabled)])?;`
/// For an iterator of matching types, use `.er_collect_all()`.
#[macro_export]
macro_rules! er_try {
    (|_| $fields:expr, $($results:tt)+) => {
        $crate::er_try!($crate::internal::factory(|()| $fields), $($results)+)
    };
    ($top:expr, [] $(,)?) => {{
        $crate::er_all!($top, [])
    }};
    ($top:expr, [$($result:expr),+ $(,)?] $(,)?) => {{
        $crate::__er_try_results!($crate, [$($result),*], $top)
    }};
}
