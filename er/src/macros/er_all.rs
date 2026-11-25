/// Runs every item, keeps all the errors and drops the oks (only builds the top error if something fails).
///
/// `er_all!((), [read_port(port), read_mode(mode)])?;`
/// Works with a mixed list of errors/trees/Results, or a single Vec/iterator (`er_all!((), results)?;`).
#[macro_export]
macro_rules! er_all {
    (|_| $fields:expr, $($results:tt)+) => {
        $crate::er_all!($crate::er_internal::factory(|()| $fields), $($results)+)
    };
    ($top:expr, [$($result:expr),* $(,)?] $(,)?) => {{
        $crate::__er_all_results!($crate, [$($result),*], $top)
    }};
    ($top:expr, $results:expr $(,)?) => {{
        $crate::__er_all_results!($crate, ($crate::er_internal::aggregate::gather_items(
            0,
            $crate::er_internal::aggregate::ErAggregateItems::er_items($results),
        )), $top)
    }};
}
