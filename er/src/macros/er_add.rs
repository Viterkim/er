/// Like `er_all!`, but adds below the error or tree you already have.
/// Returns that value even if nothing failed.
///
/// `let error = er_add!(error, [rollback(), cleanup()]);`
/// Or pass one error or Result, a collection like Vec or an iterator.
#[macro_export]
macro_rules! er_add {
    ($top:expr, [] $(,)?) => {{
        $crate::__er_add_root!($top)
    }};
    ($top:expr, [$($result:expr),* $(,)?] $(,)?) => {{
        $crate::__er_all_results!($crate, [$($result),*]; $top)
    }};
    ($top:expr, $results:expr $(,)?) => {{
        let top = $top;
        let items = $crate::er_internal::aggregate::ErAggregateItems::er_items($results);
        let mut root = $crate::__er_add_root!(top);
        $crate::er_internal::ErAggregateAdd::add(0, &mut root.nodes, items);
        root
    }};
}

#[doc(hidden)]
#[macro_export]
macro_rules! __er_add_root {
    ($top:expr) => {{
        use $crate::er_internal::{ErAddReady as _, ErNewPass as _};

        $crate::er_internal::ErNew($top)
            .__er_add_pending()
            .__er_tree()
            .__er_add_ready()
    }};
}
