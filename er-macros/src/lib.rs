#![warn(elided_lifetimes_in_paths)]
#![doc = include_str!("../README.md")]

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

#[proc_macro_derive(Er, attributes(er))]
/// Makes Error, matching Display/Debug, and public constructors.
/// Structs also work with `.er(|| path)`, so you don't have to name the error again there.
/// `#[er(no_constructors)]` turns off the constructors and this field conversion for that type.
///
/// Options: `format`, `skip`, `censor`, `exact`, `no_constructors`, `wrap`, and `crate`.
///
/// If you need `wrap` to implement a trait on the type [read this](https://github.com/Viterkim/er/blob/main/er/docs/macros.md#wrap).
/// You generally do NOT need wrap, just use `.er` on anything you see.
pub fn derive_er(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    match generate::expand(&input) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

/// Makes matching Display/Debug and public constructors but WITHOUT Error.
///
/// Options: `format`, `skip`, `censor`, `exact`, and `no_constructors`.
#[proc_macro_derive(ErFormat, attributes(er))]
pub fn derive_er_format(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    match format::expand(&input) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.to_compile_error().into(),
    }
}
