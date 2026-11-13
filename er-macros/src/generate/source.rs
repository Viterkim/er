use crate::{
    input::{
        Input,
        attrs::SourceMode,
        bounds::{
            FormatBounds, bound_key,
            impls::{is_qualified_self, is_self_path},
        },
    },
    names::{binding, plain},
};
use proc_macro2::TokenStream;
use quote::quote;
use std::collections::HashSet;
use syn::{Fields, GenericArgument, Generics, PathArguments, Type, parse_quote, visit::Visit as _};

pub fn implementation(
    input: &Input<'_>,
    formatting: &Generics,
    er_path: &syn::Path,
    bindings: &mut Vec<syn::Ident>,
) -> TokenStream {
    let name = &input.item.ident;
    if !input
        .cases
        .iter()
        .flat_map(|case| &case.fields)
        .any(|field| field.options.source.is_some())
    {
        let (impl_generics, type_generics, where_clause) = formatting.split_for_impl();
        return quote! {
            impl #impl_generics ::core::error::Error for #name #type_generics #where_clause {}
        };
    }

    let type_name = plain(name);
    let source = binding(&input.names, "__er_source");
    bindings.push(source.clone());

    let source_ref = binding(&input.names, "__ErSourceRef");
    let as_dyn = quote!(#er_path::impls::source::ErAsDynError);

    let mut parameters = input.type_names.clone();
    parameters.extend(input.const_names.iter().cloned());
    parameters.extend(
        input
            .item
            .generics
            .lifetimes()
            .map(|param| plain(&param.lifetime.ident)),
    );
    let mut generics = formatting.clone();
    let mut arms = Vec::new();
    let mut bounded = HashSet::new();
    let mut no_source = false;

    for case in &input.cases {
        let Some((index, field)) = case
            .fields
            .iter()
            .enumerate()
            .find(|(_, field)| field.options.source.is_some())
        else {
            no_source = true;
            continue;
        };

        let target = match case.variant {
            Some(variant) => quote!(Self::#variant),
            None => quote!(Self),
        };
        let pattern = match case.shape {
            Fields::Named(_) => {
                let field = &field.item.ident;
                quote!(#target { #field: #source, .. })
            }
            Fields::Unnamed(_) => {
                let fields = (0..case.fields.len()).map(|position| {
                    if position == index {
                        quote!(#source)
                    } else {
                        quote!(_)
                    }
                });
                quote!(#target(#(#fields),*))
            }
            Fields::Unit => target,
        };
        let mode = field
            .options
            .source
            .map_or(SourceMode::Normal, |source| source.mode);
        let (ty, converted, pointer) =
            value(&field.item.ty, quote!(#source), &source, &source_ref, mode);
        if let Some(pointer) = pointer {
            generics
                .make_where_clause()
                .predicates
                .push(parse_quote!(#pointer: ::core::ops::Deref));
        }
        let bound = if matches!(&ty, Type::TraitObject(_)) {
            None
        } else if matches!(&ty, Type::Path(path) if path.qself.is_none()
            && (is_self_path(&path.path, &type_name) || is_qualified_self(&path.path, &type_name)))
        {
            Some(quote!('static))
        } else {
            let mut types = Vec::new();
            let mut bounds = FormatBounds::new(&type_name, &parameters, &mut types, false);
            bounds.visit_type(&ty);
            bounds.uses_parameters.then(|| quote!(#as_dyn + 'static))
        };
        if let Some(bound) = bound
            && bound_key(&ty).is_none_or(|key| bounded.insert(key))
        {
            generics
                .make_where_clause()
                .predicates
                .push(parse_quote!(#ty: #bound));
        }
        arms.push(quote!(#pattern => #converted));
    }

    if no_source {
        arms.push(quote!(_ => ::core::option::Option::None));
    }

    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();
    quote! {
        impl #impl_generics ::core::error::Error for #name #type_generics #where_clause {
            fn source(&self) -> ::core::option::Option<&(dyn ::core::error::Error + 'static)> {
                use #er_path::impls::source::{ErAsSource as _, ErSourceOption as _, ErSourceRef as #source_ref};
                match self { #(#arms),* }
            }
        }
    }
}

pub fn value(
    ty: &Type,
    reference: TokenStream,
    source: &syn::Ident,
    source_ref: &syn::Ident,
    mode: SourceMode,
) -> (Type, TokenStream, Option<Type>) {
    match ty {
        Type::TraitObject(_) => {
            return (
                ty.clone(),
                quote!(::core::option::Option::Some(#reference)),
                None,
            );
        }
        Type::Paren(ty) => return value(&ty.elem, reference, source, source_ref, mode),
        Type::Group(ty) => return value(&ty.elem, reference, source, source_ref, mode),
        Type::Reference(ty) => {
            return value(
                &ty.elem,
                quote!(&**#reference),
                source,
                source_ref,
                SourceMode::Normal,
            );
        }
        _ => (),
    }

    if let Type::Path(path) = ty
        && path.qself.is_none()
        && let Some(segment) = path.path.segments.last()
        && let PathArguments::AngleBracketed(arguments) = &segment.arguments
        && let Some(GenericArgument::Type(inner)) = arguments.args.first()
        && (path.path.segments.len() == 1
            || matches!(
                path.path
                    .segments
                    .first()
                    .map(|segment| plain(&segment.ident))
                    .as_deref(),
                Some("std" | "alloc" | "core")
            ))
    {
        match plain(&segment.ident).as_str() {
            "Box" | "Rc" | "Arc" => {
                return value(
                    inner,
                    quote!(::core::ops::Deref::deref(#reference)),
                    source,
                    source_ref,
                    SourceMode::Normal,
                );
            }
            "Option" => {
                let (ty, inner, pointer) = value(inner, quote!(#source), source, source_ref, mode);
                return (
                    ty,
                    quote! {
                        match #reference {
                            ::core::option::Option::Some(#source) => #inner,
                            ::core::option::Option::None => ::core::option::Option::None,
                        }
                    },
                    pointer,
                );
            }
            _ => (),
        }
    }

    if mode == SourceMode::Deref {
        let target = parse_quote!(<#ty as ::core::ops::Deref>::Target);
        let (ty_target, converted, _) = value(
            &target,
            quote!(::core::ops::Deref::deref(#reference)),
            source,
            source_ref,
            SourceMode::Normal,
        );
        return (ty_target, converted, Some(ty.clone()));
    }

    (
        ty.clone(),
        quote! {{
            let #source_ref(#source) = (&#source_ref(#reference)).__er_source_option()?;
            #source_ref::__er_finish((&#source_ref(#source)).__er_as_source())
        }},
        None,
    )
}
