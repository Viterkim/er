use crate::fields::with_fields;
use crate::generate::constructors::exact_type;
use crate::input::Input;
use crate::names::{binding, plain};
use proc_macro2::{TokenStream, TokenTree};
use quote::quote;
use std::collections::HashSet;
use syn::Data;

pub fn construct(input: &Input<'_>) -> TokenStream {
    if input.options.no_constructors {
        return TokenStream::new();
    }
    if !matches!(&input.item.data, Data::Struct(_)) || input.cases[0].fields.is_empty() {
        return TokenStream::new();
    }

    let case = &input.cases[0];
    let name = &input.item.ident;
    let mut generics = input.item.generics.clone();
    let (_, type_generics, _) = input.item.generics.split_for_impl();
    let mut reserved = HashSet::new();
    let item = input.item;
    let mut tokens: Vec<_> = quote!(#item).into_iter().collect();
    while let Some(token) = tokens.pop() {
        match token {
            TokenTree::Ident(ident) => {
                reserved.insert(plain(&ident));
            }
            TokenTree::Group(group) => tokens.extend(group.stream()),
            _ => {}
        }
    }

    let mut types = Vec::with_capacity(case.fields.len());
    let mut bindings = Vec::with_capacity(case.fields.len());
    let mut values = Vec::with_capacity(case.fields.len());

    for (index, field) in case.fields.iter().enumerate() {
        let ty = &field.item.ty;
        let value_ident = binding(&reserved, &format!("__er_value_{index}"));
        reserved.insert(plain(&value_ident));

        if field.options.exact || exact_type(ty, &input.type_names) {
            types.push(quote!(#ty));
            values.push(quote!(#value_ident));
        } else {
            let generic = binding(&reserved, &format!("__ErInput{index}"));
            reserved.insert(plain(&generic));
            generics
                .params
                .push(syn::parse_quote!(#generic: ::core::convert::Into<#ty>));
            types.push(quote!(#generic));
            values.push(quote!(::core::convert::Into::into(#value_ident)));
        }
        bindings.push(value_ident);
    }

    let payload = match types.as_slice() {
        [] => quote!(()),
        [ty] => quote!(#ty),
        _ => quote!((#(#types),*)),
    };
    let pattern = match bindings.as_slice() {
        [] => quote!(()),
        [binding] => quote!(#binding),
        _ => quote!((#(#bindings),*)),
    };
    let body = with_fields(case, quote!(Self), &values);
    let (impl_generics, _, where_clause) = generics.split_for_impl();

    // The extra tuple keeps this clear of From<T> for T, even for boxed fields.
    quote! {
        impl #impl_generics ::core::convert::From<(#payload,)> for #name #type_generics #where_clause {
            fn from((#pattern,): (#payload,)) -> Self {
                #body
            }
        }
    }
}
