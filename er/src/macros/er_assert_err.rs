/// Get the error back, or fail the test if it was Ok.
/// With a pattern, check which error you got and return `()`.
///
/// `er_assert_err!(load(path), LoadErr::TooLarge { size } if *size > 10);`
#[macro_export]
macro_rules! er_assert_err {
    ($result:expr $(,)?) => {{
        $crate::impls::testing::expect_error($result)?
    }};
    ($result:expr, $pattern:pat $(if $guard:expr)? $(,)?) => {{
        $crate::__er_assert_err!($crate, $result, $pattern $(if $guard)?)
    }};
}
