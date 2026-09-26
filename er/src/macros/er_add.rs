/// Like `er_all!`, but adds below the tree you already have.
/// Returns that tree even if nothing failed.
///
/// `let error = er_add!(error, [rollback(), cleanup()]);`
/// Or pass a collection of results: `er_add!(error, results)`.
#[macro_export]
macro_rules! er_add {
    ($tree:expr, [$($result:expr),* $(,)?] $(,)?) => {{
        let __er_tree = $tree;
        let __er_results: [::core::result::Result<(), $crate::ErPart>; _] =
            $crate::__er_all_results!($crate::ErAllItem, [$($result),*]);
        $crate::er_add!(__er_tree, __er_results)
    }};
    ($tree:expr, $results:expr $(,)?) => {{
        $crate::ErTree::er_add(
            $tree,
            ::core::iter::Iterator::filter_map(
                ::core::iter::IntoIterator::into_iter($results),
                ::core::result::Result::err,
            ),
        )
    }};
}
