/// Return this error now. Existing trees keep their children and traces.
#[macro_export]
macro_rules! er_bail {
    ($error:expr $(,)?) => {{
        return ::core::result::Result::Err(::core::convert::Into::into($error));
    }};
}
