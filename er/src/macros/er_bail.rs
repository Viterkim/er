/// Return this error now. Existing trees keep their children and traces.
#[macro_export]
macro_rules! er_bail {
    ($error:expr $(,)?) => {{
        return ::core::result::Result::Err(::core::convert::Into::into($error));
    }};
}

/// Bail if the condition is true. Only makes the error if needed.
#[macro_export]
macro_rules! er_bail_if {
    ($error:expr, $condition:expr $(,)?) => {{
        if $condition {
            $crate::er_bail!($error);
        }
    }};
}

/// Bail unless the condition is true. Only makes the error if needed.
#[macro_export]
macro_rules! er_bail_unless {
    ($error:expr, $condition:expr $(,)?) => {{
        if !$condition {
            $crate::er_bail!($error);
        }
    }};
}
