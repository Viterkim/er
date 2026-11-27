pub mod body;

use crate::generate::{construct, impl_scope, warnings::warnings};
use crate::input::{Input, attrs::FieldMode, bounds::format_generics};
use crate::names::{binding, shield_bindings};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{DeriveInput, Error, Generics, Ident, Result};

pub fn expand(item: &DeriveInput) -> Result<TokenStream> {
    let input = Input::parse(item)?;
    for field in input.cases.iter().flat_map(|case| &case.fields) {
        if field.options.source.is_some() {
            return Err(Error::new_spanned(
                field.item,
                "`source` needs `#[derive(Er)]`, an `ErFormat` type isn't an error, so it has no source",
            ));
        }
    }
    if input.options.wrap.is_some() {
        return Err(Error::new_spanned(
            &item.ident,
            "`wrap` needs `#[derive(Er)]`, `ErFormat` can't make a Wrap",
        ));
    }

    let generics = format_generics(&input);
    let formatter = binding(&input.names, "__er_f");
    let mut bindings = Vec::new();
    let formatting = implementations(&input, &generics, &formatter, &mut bindings);

    let needs_fields = input
        .cases
        .iter()
        .flat_map(|case| &case.fields)
        .any(|field| field.options.default.is_some() || field.options.src_location);
    let (constructor_impls, constructors) =
        construct::implementations(&input, needs_fields, &mut bindings)?;
    let shields = shield_bindings(&bindings);
    let warnings = warnings(&input);

    let mut output = impl_scope([shields, formatting, constructor_impls, warnings]);
    output.extend(constructors);
    Ok(output)
}

pub fn implementations(
    input: &Input<'_>,
    generics: &Generics,
    formatter: &Ident,
    bindings: &mut Vec<Ident>,
) -> TokenStream {
    let name = &input.item.ident;
    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();
    bindings.push(formatter.clone());
    for case in &input.cases {
        for argument in case
            .formats
            .templates()
            .flat_map(|format| &format.arguments)
        {
            if argument.optional {
                let name = format!("__er_some_{}", argument.field);
                bindings.push(binding(&input.names, &name));
            }
        }

        if case.variant.is_none() {
            continue;
        }

        for (index, field) in case.fields.iter().enumerate() {
            if field.options.mode == FieldMode::Normal && field.used_in_format {
                bindings.push(binding(&input.names, &format!("__er_{index}")));
            }
        }
    }
    let display_body = body::display(input, formatter);
    let debug_body = body::debug(input, formatter);

    quote! {
        impl #impl_generics ::core::fmt::Debug for #name #type_generics #where_clause {
            fn fmt(&self, #formatter: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                #debug_body
            }
        }
        impl #impl_generics ::core::fmt::Display for #name #type_generics #where_clause {
            fn fmt(&self, #formatter: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                #display_body
            }
        }
    }
}
