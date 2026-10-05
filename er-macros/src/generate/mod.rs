use crate::{
    format,
    input::{Input, bounds::format_generics},
    names::binding,
};
use construct::construct;
use constructors::constructors;
use proc_macro_crate::{FoundCrate, crate_name};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{DeriveInput, Path, parse_quote};

pub mod construct;
pub mod constructors;
pub mod into;
pub mod replace_self;
pub mod source;
pub mod wrap;

pub fn expand(item: &DeriveInput) -> syn::Result<TokenStream> {
    expand_for(item, "er")
}

pub fn expand_for(item: &DeriveInput, package: &str) -> syn::Result<TokenStream> {
    let input = Input::parse(item)?;
    let generics = format_generics(&input)?;
    let path = match &input.options.er_path {
        Some(path) => path.clone(),
        None => runtime_path(package)?,
    };

    let formatter = binding(&input.const_names, "__er_f");
    let formatting = format::implementations(&input, &generics, &formatter);
    let constructors = constructors(&input)?;

    let construct = construct(&input);
    let error = source::implementation(&input, &generics);
    let into = into::implementation(&input, &path)?;

    let wrap = match &input.options.wrap {
        Some(options) => wrap::expand(item, &path, options, &input.const_names, &formatter)?,
        None => TokenStream::new(),
    };

    Ok(quote! {
        #formatting
        #error
        #constructors
        #construct
        #into
        #wrap
    })
}

pub fn runtime_path(package: &str) -> syn::Result<Path> {
    match crate_name(package) {
        Ok(FoundCrate::Itself) => Ok(parse_quote!(crate)),
        Ok(FoundCrate::Name(name)) => syn::parse_str(&format!("::{name}")),
        Err(_) => syn::parse_str(&format!("::{}", package.replace('-', "_"))),
    }
}
