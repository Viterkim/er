/// Run every result in order and return their values as a tuple if all succeed.
/// If anything fails, keep every error and drop the oks. The top error is only built on failure.
///
/// `let (port, enabled) = er_try!((), [read_port(port), read_enabled(enabled)])?;`
/// For an iterator of matching types, use `.er_collect_all()`.
#[macro_export]
macro_rules! er_try {
    (|_| $fields:expr, $($results:tt)+) => {
        $crate::er_try!($crate::er_internal::factory(|()| $fields), $($results)+)
    };
    ($top:expr, [] $(,)?) => {{
        $crate::er_all!($top, [])
    }};
    ($top:expr, [$($result:expr),+ $(,)?] $(,)?) => {{
        $crate::__er_try_results!($crate, [$($result),*], $top)
    }};
}
