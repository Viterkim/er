pub mod construct;
pub mod into;
pub mod replace_self;
pub mod source;
pub mod wrap;

use crate::{
    format,
    input::{Input, bounds::format_generics},
    names::binding,
};
use proc_macro_crate::{FoundCrate, crate_name};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{DeriveInput, Path, parse_quote};

pub fn expand(item: &DeriveInput) -> syn::Result<TokenStream> {
    expand_for(item, "er")
}

pub fn expand_for(item: &DeriveInput, package: &str) -> syn::Result<TokenStream> {
    let input = Input::parse(item)?;
    let generics = format_generics(&input);
    let formatter = binding(&input.const_names, "__er_f");
    let formatting = format::implementations(&input, &generics, &formatter);
    let constructors = construct::methods(&input)?;

    let fields = construct::from_fields(&input);
    let error = source::implementation(&input, &generics);
    let (into, wrap) = if input.options.wrap.is_some()
        || input
            .cases
            .iter()
            .flat_map(|case| &case.fields)
            .any(|field| field.options.into.is_some())
    {
        let path = match &input.options.er_path {
            Some(path) => path.clone(),
            None => runtime_path(package)?,
        };
        let into = into::implementation(&input, &path)?;
        let wrap = match &input.options.wrap {
            Some(options) => wrap::expand(item, &path, options, &input.const_names, &formatter),
            None => TokenStream::new(),
        };

        (into, wrap)
    } else {
        (TokenStream::new(), TokenStream::new())
    };

    Ok(quote! {
        #formatting
        #error
        #constructors
        #fields
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
