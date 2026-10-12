pub mod body;
pub mod censor;

use crate::generate::construct;
use crate::input::{Input, attrs::FieldMode, bounds::format_generics};
use crate::names::{binding, shield_bindings};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{DeriveInput, Error, Fields, Generics, Ident, Result};

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

    let generics = format_generics(&input);
    let formatter = binding(&input.names, "__er_f");
    let formatting = implementations(&input, &generics, &formatter);
    let constructors = construct::methods(&input)?;
    Ok(quote! {
        #formatting
        #constructors
    })
}

pub fn implementations(input: &Input<'_>, generics: &Generics, formatter: &Ident) -> TokenStream {
    let name = &input.item.ident;
    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();
    let debug = binding(&input.names, "__er_d");
    let mut locals = vec![formatter.clone()];
    if input.cases.iter().any(|case| match case.shape {
        Fields::Unit => false,
        Fields::Named(_) | Fields::Unnamed(_) => case.format.is_none(),
    }) {
        locals.push(debug.clone());
    }
    for case in &input.cases {
        if case.variant.is_none() {
            continue;
        }

        for (index, field) in case.fields.iter().enumerate() {
            if field.options.mode == FieldMode::Normal
                && (!field.formats.is_empty() || field.sets_width_or_precision)
            {
                locals.push(binding(&input.names, &format!("__er_{index}")));
            }
        }
    }
    let shields = shield_bindings(&locals);
    let body = body::debug_body(input, formatter, &debug);
    let censor = censor::helper(input, formatter);

    quote! {
        const _: () = {
            #shields

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
        };
    }
}
