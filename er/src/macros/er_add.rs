/// Like `er_all!`, but adds below the tree you already have.
/// Returns that tree even if nothing failed.
///
/// `let error = er_add!(error, [rollback(), cleanup()]);`
/// Or pass one error or Result, a collection like Vec or an iterator.
#[macro_export]
macro_rules! er_add {
    ($tree:expr, [] $(,)?) => {{
        $crate::IntoErTree::into_er_tree($tree)
    }};
    ($tree:expr, [$($result:expr),* $(,)?] $(,)?) => {{
        $crate::aggregate::add_parts($tree, &mut $crate::__er_all_results!($crate, [$($result),*]))
    }};
    ($tree:expr, $results:expr $(,)?) => {{
        $crate::internal::ErAggregateAdd::add(
            0, $tree, $crate::aggregate::ErAggregateItems::er_items($results),
        )
    }};
}
