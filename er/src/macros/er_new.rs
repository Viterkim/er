/// Make a tree value with your error on top. Does not return from the function.
/// Add an error, tree, Result or a list of them as the second argument. Successful Results are skipped.
///
/// `let error = er_new!(ImportErr::rejected(path), [rollback(), cleanup()]);`
/// With just the top, `er_new!(ImportErr::rejected(path))` starts a tree you can add to later.
#[macro_export]
macro_rules! er_new {
    () => {
        $crate::er_new!(())
    };
    (|_| $fields:expr $(,)?) => {{
        ::core::convert::identity::<$crate::ErTree<_>>(
            $crate::er_internal::ErBailFields::er_bail_fields(
                $crate::er_internal::factory(|()| $fields)(()),
            ),
        )
    }};
    (|_| $fields:expr, [$($result:expr),* $(,)?] $(,)?) => {{
        $crate::er_add!($crate::er_new!(|_| $fields), [$($result),*])
    }};
    (|_| $fields:expr, $results:expr $(,)?) => {{
        $crate::er_add!($crate::er_new!(|_| $fields), $results)
    }};
    (|| $top:expr $(,)?) => {{
        $crate::er_new!($crate::er_internal::ready_factory(|| $top))
    }};
    (|| $top:expr, $($results:tt)+) => {
        $crate::er_new!($crate::er_internal::ready_factory(|| $top), $($results)+)
    };
    ($top:expr $(,)?) => {{
        #[allow(unused_imports)]
        use $crate::er_internal::{ErNewFields as _, ErNewPass as _};

        // An existing Wrap carries its own tree, including when a factory hands it back.
        ::core::convert::identity::<$crate::ErTree<_>>(
            $crate::er_internal::ErNew($top)
                .__er_tree()
                .__er_factory()
                .__er_tree()
                .__er_finish(),
        )
    }};
    ($top:expr, [$($result:expr),* $(,)?] $(,)?) => {{
        $crate::er_add!($crate::er_new!($top), [$($result),*])
    }};
    ($top:expr, $results:expr $(,)?) => {{
        $crate::er_add!($crate::er_new!($top), $results)
    }};
}
