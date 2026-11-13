/// Evaluate every item, keep every error and drop the successful values.
/// Build the top error only if something fails.
///
/// `er_all!((), [read_port(port), read_mode(mode)])?;`
/// Pass one error or Result, a collection like Vec or an iterator the same way, `er_all!((), results)?;`
/// A list can mix bare errors, existing trees and Results. Use an Option for a check that can be absent.
#[macro_export]
macro_rules! er_all {
    (|_| $fields:expr, $($results:tt)+) => {
        $crate::er_all!($crate::internal::factory(|()| $fields), $($results)+)
    };
    ($top:expr, [$($result:expr),* $(,)?] $(,)?) => {{
        $crate::__er_all_results!($crate, [$($result),*], $top)
    }};
    ($top:expr, $results:expr $(,)?) => {{
        $crate::__er_all_results!($crate, ($crate::aggregate::gather_items(
            0,
            $crate::aggregate::ErAggregateItems::er_items($results),
        )), $top)
    }};
}
