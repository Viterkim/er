use crate::input::Case;
use crate::names::plain;
use proc_macro2::{TokenStream, TokenTree};
use quote::quote;
use syn::{Fields, GenericArgument, PathArguments, Type};

pub fn option_inner(mut ty: &Type) -> Option<&Type> {
    // A type handed over by another macro's `$ty:ty` arrives in an invisible group, `(Option<T>)` in a real one.
    loop {
        ty = match ty {
            Type::Group(group) => &group.elem,
            Type::Paren(paren) => &paren.elem,
            _ => break,
        };
    }

    let Type::Path(path) = ty else {
        return None;
    };
    let segments = &path.path.segments;
    let segment = segments.last()?;

    if path.qself.is_some() || plain(&segment.ident) != "Option" {
        return None;
    }

    // `Option<T>` or the std path to it, `other::Option<T>` is somebody else's type.
    let from_std = matches!(plain(&segments[0].ident).as_str(), "std" | "core" | "alloc");
    if segments.len() > 1 && !from_std {
        return None;
    }

    let PathArguments::AngleBracketed(arguments) = &segment.arguments else {
        return None;
    };

    match arguments.args.first() {
        Some(GenericArgument::Type(inner)) => Some(inner),
        _ => None,
    }
}

// Works for both construction and matching.
pub fn with_fields(case: &Case<'_>, target: TokenStream, values: &[TokenStream]) -> TokenStream {
    match case.shape {
        Fields::Unit => target,
        Fields::Unnamed(_) => quote!(#target(#(#values),*)),
        Fields::Named(_) => {
            let fields = case.fields.iter().zip(values).map(|(field, value)| {
                let name = &field.item.ident;
                let mut tokens = value.clone().into_iter();

                // Keep the derive from triggering redundant_field_names on the user's fields.
                if let Some(TokenTree::Ident(value_name)) = tokens.next()
                    && tokens.next().is_none()
                    && name.as_ref() == Some(&value_name)
                {
                    quote!(#name)
                } else {
                    quote!(#name: #value)
                }
            });
            quote!(#target { #(#fields),* })
        }
    }
}
