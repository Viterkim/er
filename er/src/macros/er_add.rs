/// Like `er_all!`, but adds below the tree you already have.
/// Returns that tree even if nothing failed.
///
/// `let error = er_add!(error, [rollback(), cleanup()]);`
/// Or pass a collection of results: `er_add!(error, results)`.
#[macro_export]
macro_rules! er_add {
    ($tree:expr, [$($result:expr),* $(,)?] $(,)?) => {{
        $crate::er_add!(
            $tree,
            $crate::__er_all_results!($crate, [$($result),*])
                as [::core::result::Result<(), $crate::ErPart>; _],
        )
    }};
    ($tree:expr, $results:expr $(,)?) => {{
        $crate::IntoErTree::er_add(
            $tree,
            ::core::iter::Iterator::filter_map(
                ::core::iter::IntoIterator::into_iter($results),
                {
                    #[allow(unused_imports)]
                    use ::core::mem::drop as __er_item;

                    |__er_item| $crate::ErAllItem::er_all_item(__er_item).err()
                },
            ),
        )
    }};
}
