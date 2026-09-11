/// Keep every error, drop oks. The parent only gets made if something fails.
///
/// `er_all!(ConfigEr::new, [read_port(port), read_mode(mode)])?;`
/// An existing collection works too: `er_all!(ConfigEr::new, results)?;`
#[macro_export]
macro_rules! er_all {
    ($parent:expr, [$($result:expr),* $(,)?] $(,)?) => {{
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
        $crate::aggregate::collect(|| ($parent)(), __er_results)
    }};
    ($parent:expr, $results:expr $(,)?) => {{
        $crate::aggregate::collect(|| ($parent)(), $results)
    }};
}

// Turns into something like (types can be different):
//
// let mut errors = Vec::new();
// for result in results {
//     match result {
//         Ok(value) => drop(value),
//         Err(error) => errors.push(error.into_er_node()),
//     }
// }
//
// if errors.is_empty() {
//     Ok(())
// } else {
//     Err(ErTree::new(parent(), errors))
// }
