use crate::fields::with_fields;
use crate::input::{Case, Input};
use crate::names::{binding, plain, snake_case};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use std::collections::HashSet;
use syn::{Ident, Type};

pub fn constructors(input: &Input<'_>) -> syn::Result<TokenStream> {
    let name = &input.item.ident;
    let (impl_generics, type_generics, where_clause) = input.item.generics.split_for_impl();
    let variants: HashSet<_> = input
        .cases
        .iter()
        .filter_map(|case| case.variant.map(plain))
        .collect();
    let mut methods = Vec::with_capacity(input.cases.len());
    let mut method_names = HashSet::new();

    for case in &input.cases {
        let (method, target) = if let Some(variant) = case.variant {
            let text = snake_case(&plain(variant));
            if text == "er_wrap" && input.options.wrap.is_some() {
                return Err(syn::Error::new_spanned(
                    variant,
                    "constructor `er_wrap` clashes with the generated Wrap method",
                ));
            }

            if !method_names.insert(text.clone()) || variants.contains(&text) {
                return Err(syn::Error::new_spanned(
                    variant,
                    format!("constructor `{text}` clashes with another variant or constructor"),
                ));
            }

            let method = Ident::new_raw(&text, variant.span());
            (method, quote!(Self::#variant))
        } else {
            (format_ident!("new"), quote!(Self))
        };

        let (parameters, values) = constructor_arguments(input, case);

        let body = with_fields(case, target, &values);

        methods.push(quote! {
            #[must_use]
            pub fn #method(#(#parameters),*) -> Self {
                #body
            }
        });
    }

    Ok(quote! {
        impl #impl_generics #name #type_generics #where_clause {
            #(#methods)*
        }
    })
}

pub fn constructor_arguments(
    input: &Input<'_>,
    case: &Case<'_>,
) -> (Vec<TokenStream>, Vec<TokenStream>) {
    let mut reserved = input.const_names.clone();
    for field in &case.fields {
        if let Some(name) = &field.item.ident {
            reserved.insert(plain(name));
        }
    }

    let mut parameters = Vec::with_capacity(case.fields.len());
    let mut values = Vec::with_capacity(case.fields.len());
    for (index, field) in case.fields.iter().enumerate() {
        let ty = &field.item.ty;
        let exact = field.options.exact || exact_type(ty, &input.type_names);
        let mut argument = match &field.item.ident {
            Some(name) => name.clone(),
            None if case.fields.len() == 1 => format_ident!("value"),
            None => format_ident!("value{index}"),
        };
        let text = plain(&argument);
        if snake_case(&text) != text || input.const_names.contains(&text) {
            argument = binding(&reserved, &format!("__er_arg_{index}"));
        }
        reserved.insert(plain(&argument));

        if exact {
            parameters.push(quote!(#argument: #ty));
            values.push(quote!(#argument));
        } else {
            parameters.push(quote!(#argument: impl ::core::convert::Into<#ty>));
            values.push(quote!(::core::convert::Into::into(#argument)));
        }
    }

    (parameters, values)
}

pub fn exact_type(ty: &Type, type_names: &HashSet<String>) -> bool {
    match ty {
        Type::Paren(ty) => return exact_type(&ty.elem, type_names),
        Type::Group(ty) => return exact_type(&ty.elem, type_names),
        _ => (),
    }

    if matches!(
        ty,
        Type::FnPtr(_)
            | Type::Reference(_)
            | Type::Ptr(_)
            | Type::Tuple(_)
            | Type::Array(_)
            | Type::Slice(_)
    ) {
        return true;
    }

    let Type::Path(path) = ty else {
        return false;
    };
    if path.qself.is_some() {
        return false;
    }
    if let Some(ident) = path.path.get_ident()
        && type_names.contains(&plain(ident))
    {
        return true;
    }

    let segments = &path.path.segments;
    let ordinary = segments.len() == 1;
    let primitive_path = segments.len() == 3
        && matches!(plain(&segments[0].ident).as_str(), "core" | "std")
        && plain(&segments[1].ident) == "primitive";
    if !ordinary && !primitive_path {
        return false;
    }
    let Some(segment) = segments.last() else {
        return false;
    };

    matches!(
        plain(&segment.ident).as_str(),
        "bool"
            | "char"
            | "i8"
            | "i16"
            | "i32"
            | "i64"
            | "i128"
            | "isize"
            | "u8"
            | "u16"
            | "u32"
            | "u64"
            | "u128"
            | "usize"
            | "f32"
            | "f64"
    )
}
