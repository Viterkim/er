use core::fmt::{Debug, Formatter, Result};

#[inline(never)]
pub fn debug_struct(
    formatter: &mut Formatter<'_>,
    name: &str,
    fields: &[(&str, &dyn Debug)],
) -> Result {
    let mut builder = formatter.debug_struct(name);
    for &(field, value) in fields {
        builder.field(field, value);
    }

    builder.finish()
}

#[inline(never)]
pub fn debug_field(
    formatter: &mut Formatter<'_>,
    (name, (field, value)): (&str, (&str, &dyn Debug)),
) -> Result {
    formatter.debug_struct(name).field(field, value).finish()
}

#[inline(never)]
pub fn debug_tuple(formatter: &mut Formatter<'_>, name: &str, fields: &[&dyn Debug]) -> Result {
    let mut builder = formatter.debug_tuple(name);
    for &value in fields {
        builder.field(value);
    }

    builder.finish()
}

#[inline(never)]
pub fn debug_tuple_field(
    formatter: &mut Formatter<'_>,
    (name, value): (&str, &dyn Debug),
) -> Result {
    formatter.debug_tuple(name).field(value).finish()
}

pub struct Censored;

macro_rules! censored {
    ($($format:ident),*) => {
        $(
            impl core::fmt::$format for Censored {
                fn fmt(&self, formatter: &mut Formatter<'_>) -> Result {
                    formatter.pad("*CENSORED*")
                }
            }
        )*
    };
}
censored!(
    Display, Debug, Binary, Octal, LowerHex, UpperHex, LowerExp, UpperExp, Pointer
);
