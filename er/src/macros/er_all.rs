/// Keep every error, drop oks. The top error only gets made if something fails.
/// Does not stop early, every result is evaluated.
///
/// `er_all!((), [read_port(port), read_mode(mode)])?;`
/// An existing collection works too: `er_all!((), results)?;`
#[macro_export]
macro_rules! er_all {
    ($top:expr, [$($result:expr),* $(,)?] $(,)?) => {{
        let __er_results: [::core::result::Result<(), $crate::ErNode>; _] = [$(
            match $result {
                ::core::result::Result::Ok(__er_success) => {
                    ::core::mem::drop(__er_success);
                    ::core::result::Result::Ok(())
                },
                ::core::result::Result::Err(__er_failure) => {
                    ::core::result::Result::Err(
                        $crate::IntoErNode::into_er_node(__er_failure)
                    )
                }
            }
        ),*];
        $crate::aggregate::collect(|| $top, __er_results)
    }};
    ($top:expr, $results:expr $(,)?) => {{
        $crate::aggregate::collect(|| $top, $results)
    }};
}
