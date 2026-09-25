/// Keep every error, drop oks. The top error only gets made if something fails.
/// Does not stop early, every result is evaluated.
///
/// `er_all!((), [read_port(port), read_mode(mode)])?;`
/// An existing collection works too: `er_all!((), results)?;`
#[macro_export]
macro_rules! er_all {
    ($top:expr, [$($result:expr),* $(,)?] $(,)?) => {{
        let __er_results: [::core::result::Result<(), $crate::ErPart>; _] =
            $crate::__er_all_results!($crate::IntoErPart, [$($result),*]);
        $crate::aggregate::collect(|| $crate::ErMake::er_make($top), __er_results)
    }};
    ($top:expr, $results:expr $(,)?) => {{
        $crate::aggregate::collect(|| $crate::ErMake::er_make($top), $results)
    }};
}
