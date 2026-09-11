#![warn(elided_lifetimes_in_paths)]
#![doc = "Derive macros for [Er](https://github.com/Viterkim/er)."]

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
/// Makes Error + matching Display/Debug, and public constructors (unless you use #[er(no_constructors)]).
///
/// Options: `format`, `skip`, `censor`, `exact`, `no_constructors`, `wrap`, and `crate`.
///
/// Look in the docs for macros.md if you need 'wrap' for implementing a trait on the type.
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
