use crate::{
    input::{
        Input,
        bounds::{FormatBounds, impls::is_self_path},
    },
    names::{binding, plain},
};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{Fields, GenericArgument, Generics, PathArguments, Type, parse_quote, visit::Visit as _};

pub fn implementation(input: &Input<'_>, formatting: &Generics) -> TokenStream {
    let name = &input.item.ident;
    let type_name = plain(name);
    let source = binding(&input.const_names, "__er_source");
    let mut reserved = input.const_names.clone();
    reserved.extend(input.type_names.iter().cloned());
    reserved.insert(plain(name));
    let as_dyn = binding(&reserved, "__ErAsDynError");
    reserved.insert(plain(&as_dyn));
    let as_source = binding(&reserved, "__ErAsSource");
    reserved.insert(plain(&as_source));
    let alloc = binding(&reserved, "__er_alloc");

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
    let mut has_source = false;

    for case in &input.cases {
        let target = match case.variant {
            Some(variant) => quote!(Self::#variant),
            None => quote!(Self),
        };
        let Some((index, field)) = case
            .fields
            .iter()
            .enumerate()
            .find(|(_, field)| field.options.source)
        else {
            let pattern = match case.shape {
                Fields::Unit => target,
                Fields::Named(_) => quote!(#target { .. }),
                Fields::Unnamed(_) => quote!(#target(..)),
            };
            arms.push(quote!(#pattern => ::core::option::Option::None));
            continue;
        };

        has_source = true;
        let pattern = match case.shape {
            Fields::Named(_) => {
                let field = &field.item.ident;
                quote!(#target { #field: #source, .. })
            }
            _ => {
                let fields = (0..case.fields.len()).map(|position| {
                    if position == index {
                        quote!(#source)
                    } else {
                        quote!(_)
                    }
                });
                quote!(#target(#(#fields),*))
            }
        };
        let (ty, value) = value(&field.item.ty, quote!(#source), &source);
        if matches!(&ty, Type::Path(path) if path.qself.is_none() && is_self_path(&path.path, &plain(name)))
        {
            generics
                .make_where_clause()
                .predicates
                .push(parse_quote!(#ty: 'static));
        } else {
            let mut types = Vec::new();
            let mut bounds = FormatBounds::new(&type_name, &parameters, &mut types, false);
            bounds.visit_type(&ty);
            if bounds.uses_parameters {
                generics
                    .make_where_clause()
                    .predicates
                    .push(parse_quote!(#ty: #as_dyn + 'static));
            }
        }
        arms.push(quote!(#pattern => #value));
    }

    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();
    let method = has_source.then(|| {
        quote! {
            fn source(&self) -> ::core::option::Option<&(dyn ::core::error::Error + 'static)> {
                match self { #(#arms),* }
            }
        }
    });

    let implementation = quote! {
        impl #impl_generics ::core::error::Error for #name #type_generics #where_clause {
            #method
        }
    };
    if !has_source {
        return implementation;
    }

    let conversion = conversion(&as_dyn, &as_source, &alloc);
    quote! {
        const _: () = {
            #conversion
            #implementation
        };
    }
}

pub fn value(ty: &Type, reference: TokenStream, source: &syn::Ident) -> (Type, TokenStream) {
    match ty {
        Type::Paren(ty) => return value(&ty.elem, reference, source),
        Type::Group(ty) => return value(&ty.elem, reference, source),
        Type::Reference(ty) => return value(&ty.elem, quote!(*#reference), source),
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
        match segment.ident.to_string().as_str() {
            "Box" | "Rc" | "Arc" => {
                return value(inner, quote!(::core::ops::Deref::deref(#reference)), source);
            }
            "Option" => {
                let (ty, inner) = value(inner, quote!(#source), source);
                return (
                    ty,
                    quote! {
                        match #reference {
                            ::core::option::Option::Some(#source) => #inner,
                            ::core::option::Option::None => ::core::option::Option::None,
                        }
                    },
                );
            }
            _ => (),
        }
    }

    (
        ty.clone(),
        quote!(::core::option::Option::Some((#reference).__er_as_source())),
    )
}

pub fn conversion(as_dyn: &syn::Ident, as_source: &syn::Ident, alloc: &syn::Ident) -> TokenStream {
    let objects = [
        quote!(),
        quote!(+ ::core::marker::Send),
        quote!(+ ::core::marker::Sync),
        quote!(+ ::core::marker::Send + ::core::marker::Sync),
    ]
    .into_iter()
    .map(|bounds| {
        quote! {
            impl #as_dyn for dyn ::core::error::Error #bounds + 'static {
                fn __er_as_dyn(&self) -> &(dyn ::core::error::Error + 'static) {
                    self
                }
            }
        }
    });
    let pointers = [
        (quote!(#alloc::boxed::Box), quote!()),
        (quote!(#alloc::rc::Rc), quote!()),
        (
            quote!(#alloc::sync::Arc),
            quote!(#[cfg(target_has_atomic = "ptr")]),
        ),
    ]
    .into_iter()
    .map(|(pointer, attributes)| {
        quote! {
            #attributes
            impl<'a, T: #as_dyn + ?Sized> #as_source<'a> for &'a #pointer<T> {
                fn __er_as_source(self) -> &'a (dyn ::core::error::Error + 'static) {
                    #as_dyn::__er_as_dyn(&**self)
                }
            }
        }
    });

    quote! {
        extern crate alloc as #alloc;

        trait #as_dyn {
            fn __er_as_dyn(&self) -> &(dyn ::core::error::Error + 'static);
        }
        impl<T: ::core::error::Error + 'static> #as_dyn for T {
            fn __er_as_dyn(&self) -> &(dyn ::core::error::Error + 'static) {
                self
            }
        }
        #(#objects)*

        trait #as_source<'a> {
            fn __er_as_source(self) -> &'a (dyn ::core::error::Error + 'static);
        }
        impl<'a, T: #as_dyn + ?Sized> #as_source<'a> for &&'a T {
            fn __er_as_source(self) -> &'a (dyn ::core::error::Error + 'static) {
                #as_dyn::__er_as_dyn(*self)
            }
        }
        #(#pointers)*
    }
}
