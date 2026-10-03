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
pub mod into;
pub mod replace_self;
pub mod source;
pub mod wrap;

pub fn expand(item: &DeriveInput) -> syn::Result<TokenStream> {
    let input = Input::parse(item)?;
    let generics = format_generics(&input)?;

    let formatter = binding(&input.const_names, "__er_f");
    let formatting = format::implementations(&input, &generics, &formatter);
    let constructors = constructors(&input)?;

    let construct = construct(&input);
    let error = source::implementation(&input, &generics);
    let into = into::implementation(&input)?;

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
        #error
        #constructors
        #construct
        #into
        #wrap
    })
}
