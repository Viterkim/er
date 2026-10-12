/// Keep every error, drop oks. The top error only gets made if something fails.
/// Does not stop early, every item is evaluated.
/// Also accepts bare errors and existing trees.
///
/// `er_all!((), [read_port(port), read_mode(mode)])?;`
/// An existing collection works too: `er_all!((), results)?;`
#[macro_export]
macro_rules! er_all {
    ($top:expr, [$($result:expr),* $(,)?] $(,)?) => {{
        $crate::__er_all_results!($crate, [$($result),*], $top)
    }};
    ($top:expr, $results:expr $(,)?) => {{
        $crate::aggregate::collect(
            || $crate::ErMake::er_make($top),
            ::core::iter::Iterator::map(
                ::core::iter::IntoIterator::into_iter($results),
                {
                    #[allow(unused_imports)]
                    use ::core::mem::drop as __er_item;

                    |__er_item| $crate::ErAllItem::er_all_item(__er_item)
                },
            ),
        )
    }};
}
