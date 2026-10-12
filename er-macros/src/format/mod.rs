pub mod body;
pub mod censor;

use crate::generate::construct;
use crate::input::{
    Input,
    attrs::FieldMode,
    bounds::{format_generics, format_generics_for},
    format::Output,
};
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
        if field.options.source.is_some() {
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
    let fields = input
        .cases
        .iter()
        .flat_map(|case| &case.fields)
        .any(|field| field.options.default.is_some() || field.options.src_location)
        .then(|| construct::from_fields(&input));
    Ok(quote! {
        #formatting
        #constructors
        #fields
    })
}

pub fn implementations(input: &Input<'_>, generics: &Generics, formatter: &Ident) -> TokenStream {
    let name = &input.item.ident;
    let separate_normal = input
        .cases
        .iter()
        .any(|case| case.formats.display.is_some() || case.formats.debug.is_some());
    let alternate = input
        .cases
        .iter()
        .any(|case| case.formats.display_alt.is_some());
    let separate = separate_normal || alternate;
    let display_generics = separate.then(|| format_generics_for(input, Output::Display));
    let debug_generics = separate.then(|| format_generics_for(input, Output::Debug));
    let (display_impl, type_generics, display_where) = display_generics
        .as_ref()
        .unwrap_or(generics)
        .split_for_impl();
    let (debug_impl, _, debug_where) = debug_generics.as_ref().unwrap_or(generics).split_for_impl();
    let debug = binding(&input.names, "__er_d");
    let mut locals = vec![formatter.clone()];
    if input.cases.iter().any(|case| match case.shape {
        Fields::Unit => false,
        Fields::Named(_) | Fields::Unnamed(_) => {
            case.formats.get(Output::Display).is_none() || case.formats.get(Output::Debug).is_none()
        }
    }) {
        locals.push(debug.clone());
    }
    for case in &input.cases {
        if case.variant.is_none() {
            continue;
        }

        for (index, field) in case.fields.iter().enumerate() {
            if field.options.mode == FieldMode::Normal
                && (!field.formats.is_empty() || field.used_in_format)
            {
                locals.push(binding(&input.names, &format!("__er_{index}")));
            }
        }
    }
    let shields = shield_bindings(&locals);
    let censor = censor::helper(input, formatter);
    let display_body = if separate_normal {
        body::body(input, Output::Display, formatter, &debug)
    } else {
        quote!(::core::fmt::Debug::fmt(self, #formatter))
    };
    let display_body = if alternate {
        let alternate_body = body::body(input, Output::DisplayAlt, formatter, &debug);
        quote! {
            if #formatter.alternate() {
                #alternate_body
            } else {
                #display_body
            }
        }
    } else {
        display_body
    };
    let display_body = if separate {
        quote! { #censor #display_body }
    } else {
        display_body
    };
    let debug_body = body::body(input, Output::Debug, formatter, &debug);

    quote! {
        const _: () = {
            #shields

            impl #debug_impl ::core::fmt::Debug for #name #type_generics #debug_where {
                fn fmt(&self, #formatter: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    #censor
                    #debug_body
                }
            }
            impl #display_impl ::core::fmt::Display for #name #type_generics #display_where {
                fn fmt(&self, #formatter: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    #display_body
                }
            }
        };
    }
}
