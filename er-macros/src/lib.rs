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
/// Impls error, generates Display/Debug(as the same), makes constructor funcs.
/// Structs also work with `.er(|_| path)`, so you don't have to name the err type.
///
/// Options: `format`, `skip`, `censor`, `exact`, `no_constructors`, `wrap`, and `crate`.
///
/// Just use `.er` on anything you see.
/// If you need to implement a trait on your error type, you can use `wrap` [but you usually don't](https://github.com/Viterkim/er/blob/main/er/docs/macros.md#wrap).
pub fn derive_er(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    match generate::expand(&input) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

/// Does NOT impl error, Generates Display/Debug(as the same), makes constructor funcs.
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
