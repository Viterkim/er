use crate::{
    format,
    input::{Input, bounds::format_generics},
    names::binding,
};
use construct::construct;
use constructors::constructors;
use proc_macro2::TokenStream;
use quote::quote;
use syn::DeriveInput;

pub mod construct;
pub mod constructors;
pub mod replace_self;
pub mod wrap;

pub fn expand(item: &DeriveInput) -> syn::Result<TokenStream> {
    let input = Input::parse(item)?;
    let name = &item.ident;
    let generics = format_generics(&input)?;
    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();

    let formatter = binding(&input.const_names, "__er_f");
    let formatting = format::implementations(&input, &generics, &formatter);
    let constructors = constructors(&input)?;

    let construct = construct(&input);

    let wrap = match &input.options.wrap {
        Some(options) => {
            let path = match &input.options.er_path {
                Some(path) => path.clone(),
                None => syn::parse_str("::er")?,
            };
            wrap::expand(item, &path, options, &input.const_names, &formatter)?
        }
        None => TokenStream::new(),
    };

    Ok(quote! {
        #formatting
        impl #impl_generics ::core::error::Error for #name #type_generics #where_clause {}
        #constructors
        #construct
        #wrap
    })
}
