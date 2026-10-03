/// Return this error now. Existing trees keep their children and traces.
/// Takes the same construction helpers as `.er()`, or an error you've already made.
#[macro_export]
macro_rules! er_bail {
    ($error:expr $(,)?) => {{
        return ::core::result::Result::Err($crate::ErBail::er_bail($error));
    }};
}
