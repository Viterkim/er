pub mod body;
pub mod censor;

use crate::generate::constructors::constructors;
use crate::input::{Input, bounds::format_generics};
use crate::names::binding;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{DeriveInput, Error, Generics, Ident, Result};

pub fn expand(item: &DeriveInput) -> Result<TokenStream> {
    let input = Input::parse(item)?;
    for field in input.cases.iter().flat_map(|case| &case.fields) {
        if field.options.into.is_some() {
            return Err(Error::new_spanned(
                field.item,
                "use derive(Er) for `into_*` fields",
            ));
        }
        if field.options.source {
            return Err(Error::new_spanned(
                field.item,
                "use derive(Er) for `source`, ErFormat does not implement Error",
            ));
        }
    }
    if input.options.wrap.is_some() || input.options.er_path.is_some() {
        return Err(Error::new_spanned(
            &item.ident,
            "use derive(Er) for `wrap` and `crate` options",
        ));
    }

    let generics = format_generics(&input)?;
    let formatter = binding(&input.const_names, "__er_f");
    let formatting = implementations(&input, &generics, &formatter);
    let constructors = constructors(&input)?;
    Ok(quote! {
        #formatting
        #constructors
    })
}

pub fn implementations(input: &Input<'_>, generics: &Generics, formatter: &Ident) -> TokenStream {
    let name = &input.item.ident;
    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();
    let debug = binding(&input.const_names, "__er_d");
    let body = body::debug_body(input, formatter, &debug);
    let censor = censor::helper(input, formatter);

    quote! {
        impl #impl_generics ::core::fmt::Debug for #name #type_generics #where_clause {
            fn fmt(&self, #formatter: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                #censor
                #body
            }
        }
        impl #impl_generics ::core::fmt::Display for #name #type_generics #where_clause {
            fn fmt(&self, #formatter: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                ::core::fmt::Debug::fmt(self, #formatter)
            }
        }
    }
}
