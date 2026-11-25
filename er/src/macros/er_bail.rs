/// Return this error now. Existing trees keep their sub errors and saved reports.
/// Takes the same construction helpers as `.er()`, or an error you've already made.
/// Pass sub errors as a second argument, just as with [`er_new!`](crate::er_new).
#[macro_export]
macro_rules! er_bail {
    () => {
        $crate::er_bail!(())
    };
    (|_| $fields:expr $(,)?) => {{
        return ::core::result::Result::Err($crate::er_internal::ErBailValue::bail(
            (0, $crate::er_internal::ErFields), $crate::er_internal::factory(|()| $fields)(()),
        ));
    }};
    (|| $error:expr $(,)?) => {
        $crate::er_bail!($crate::er_internal::factory(|()| $error)(()))
    };
    ($error:expr $(,)?) => {{
        return ::core::result::Result::Err($crate::er_internal::ErBailValue::bail(
            (0, $crate::er_internal::ErReady), $error,
        ));
    }};
    (|_| $fields:expr, $($nodes:tt)+) => {
        $crate::__er_bail_nodes!($crate, ($crate::er_internal::factory(|()| $fields)), $($nodes)+)
    };
    (|| $top:expr, $($nodes:tt)+) => {
        $crate::__er_bail_nodes!($crate, ($crate::er_internal::ready_factory(|| $top)), $($nodes)+)
    };
    ($top:expr, $($nodes:tt)+) => {
        $crate::__er_bail_nodes!($crate, $top, $($nodes)+)
    };
}
