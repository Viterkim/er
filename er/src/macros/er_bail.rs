/// Return this error now. Existing trees keep their sub errors and saved reports.
/// Takes the same construction helpers as `.er()`, or an error you've already made.
/// Pass sub errors as a second argument, just as with [`er_new!`](crate::er_new).
#[macro_export]
macro_rules! er_bail {
    () => {
        $crate::er_bail!(())
    };
    (|_| $fields:expr $(,)?) => {{
        return ::core::result::Result::Err($crate::internal::ErBailValue::bail(
            (0, $crate::internal::ErFields), $crate::internal::factory(|()| $fields)(()),
        ));
    }};
    (|| $error:expr $(,)?) => {
        $crate::er_bail!($crate::internal::factory(|()| $error)(()))
    };
    ($error:expr $(,)?) => {{
        return ::core::result::Result::Err($crate::internal::ErBailValue::bail(
            (0, $crate::internal::ErReady), $error,
        ));
    }};
    (|_| $fields:expr, $($nodes:tt)+) => {
        $crate::__er_bail_nodes!($crate, ($crate::internal::factory(|()| $fields)), $($nodes)+)
    };
    (|| $top:expr, $($nodes:tt)+) => {
        $crate::__er_bail_nodes!($crate, ($crate::internal::ready_factory(|| $top)), $($nodes)+)
    };
    ($top:expr, $($nodes:tt)+) => {
        $crate::__er_bail_nodes!($crate, $top, $($nodes)+)
    };
}
