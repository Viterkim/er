#![warn(elided_lifetimes_in_paths)]
#![doc = include_str!("../README.md")]

mod er_all;
mod er_assert_err;
mod er_bail;
mod er_try;
mod fields;
mod format;
mod generate;
mod input;
mod names;

#[cfg(test)]
#[path = "../tests/unit/mod.rs"]
mod tests;

use proc_macro::TokenStream;
use syn::{DeriveInput, parse_macro_input};

#[proc_macro]
#[doc(hidden)]
pub fn __er_all_results(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as er_all::Input);
    er_all::expand(input).into()
}

#[proc_macro]
#[doc(hidden)]
pub fn __er_try_results(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as er_try::Input);
    er_try::expand(input).into()
}

#[proc_macro]
#[doc(hidden)]
pub fn __er_assert_err(input: TokenStream) -> TokenStream {
    er_assert_err::expand(input.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

#[proc_macro]
#[doc(hidden)]
pub fn __er_bail_nodes(input: TokenStream) -> TokenStream {
    er_bail::expand(input.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

#[proc_macro_derive(Er, attributes(er))]
/// Implements Error, Display and Debug and generates constructors.
/// With a struct, pass its fields to `.er(path)` and let the return type pick the error.
/// Use `.er(|_| (path, user.name()))` to run a lookup only when the call fails.
///
/// Options include `format`, `display`, `display_alt`, `debug`, `skip`, `censor`, `exact`,
/// `default`, `src_location`, `no_constructors`, `wrap` and `crate`. Use `source` to expose a cause.
/// Add `show_in_parent` if the parent's own message should print it too.
///
/// [`wrap`](https://github.com/Viterkim/er/blob/main/er/docs/macros.md#wrap) gives you a type
/// you own to implement a foreign trait on while keeping the tree. You usually don't need it.
pub fn derive_er(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    generate::expand(&input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// Generates Display, Debug and constructors for data inside an error. Leaves out the Error impl.
///
/// Supports `format`, `display`, `display_alt`, `debug`, `skip`, `censor`, `exact`, `default`,
/// `src_location`, `no_constructors` and `crate`.
#[proc_macro_derive(ErFormat, attributes(er))]
pub fn derive_er_format(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    format::expand(&input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
