/// Return this error now. Existing trees keep their children and traces.
#[macro_export]
macro_rules! er_bail {
    ($error:expr $(,)?) => {{
        return ::core::result::Result::Err($crate::ErBail::er_bail($error));
    }};
}
