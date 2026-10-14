use crate::{
    input::{
        Input,
        attrs::SourceMode,
        bounds::{FormatBounds, bound_key, impls::is_self_path},
    },
    names::{binding, plain, shield_bindings},
};
use proc_macro2::TokenStream;
use quote::quote;
use std::collections::HashSet;
use syn::{Fields, GenericArgument, Generics, PathArguments, Type, parse_quote, visit::Visit as _};

pub fn implementation(input: &Input<'_>, formatting: &Generics) -> TokenStream {
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
    let mut reserved = input.names.clone();
    let source = binding(&reserved, "__er_source");
    let shields = shield_bindings([&source]);
    let as_dyn = binding(&reserved, "__ErAsDynError");
    reserved.insert(plain(&as_dyn));
    let as_source = binding(&reserved, "__ErAsSource");
    reserved.insert(plain(&as_source));
    let source_ref = binding(&reserved, "__ErSourceRef");
    reserved.insert(plain(&source_ref));
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
    let mut bounded = HashSet::new();

    for case in &input.cases {
        let target = match case.variant {
            Some(variant) => quote!(Self::#variant),
            None => quote!(Self),
        };
        let Some((index, field)) = case
            .fields
            .iter()
            .enumerate()
            .find(|(_, field)| field.options.source.is_some())
        else {
            let pattern = match case.shape {
                Fields::Unit => target,
                Fields::Named(_) => quote!(#target { .. }),
                Fields::Unnamed(_) => quote!(#target(..)),
            };
            arms.push(quote!(#pattern => ::core::option::Option::None));
            continue;
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
        let mode = field.options.source.unwrap_or(SourceMode::Normal);
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
        } else if matches!(&ty, Type::Path(path) if path.qself.is_none() && is_self_path(&path.path, &type_name))
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

    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();
    let implementation = quote! {
        impl #impl_generics ::core::error::Error for #name #type_generics #where_clause {
            fn source(&self) -> ::core::option::Option<&(dyn ::core::error::Error + 'static)> {
                match self { #(#arms),* }
            }
        }
    };

    let conversion = conversion(&as_dyn, &as_source, &source_ref, &alloc);
    quote! {
        const _: () = {
            #shields
            #conversion
            #implementation
        };
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
        quote!(::core::option::Option::Some(#source_ref::__er_finish(
            (&#source_ref(#reference)).__er_as_source()
        ))),
        None,
    )
}

pub fn conversion(
    as_dyn: &syn::Ident,
    as_source: &syn::Ident,
    source_ref: &syn::Ident,
    alloc: &syn::Ident,
) -> TokenStream {
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
            impl<'a, T: #as_dyn + ?::core::marker::Sized> #as_source<'a> for &#source_ref<'a, #pointer<T>> {
                fn __er_as_source(self) -> #source_ref<'a, dyn ::core::error::Error> {
                    #source_ref(#as_dyn::__er_as_dyn(&**self.0))
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

        // Dispatch on our own receiver, so an error's inherent methods can't win.
        // Keep the result private too, so a foreign blanket method can't replace it.
        struct #source_ref<'a, T: ?::core::marker::Sized>(&'a T);
        impl<'a> #source_ref<'a, dyn ::core::error::Error> {
            fn __er_finish(self) -> &'a (dyn ::core::error::Error + 'static) {
                self.0
            }
        }

        trait #as_source<'a> {
            fn __er_as_source(self) -> #source_ref<'a, dyn ::core::error::Error>;
        }
        impl<'a, T: #as_dyn + ?::core::marker::Sized> #as_source<'a> for &&#source_ref<'a, T> {
            fn __er_as_source(self) -> #source_ref<'a, dyn ::core::error::Error> {
                #source_ref(#as_dyn::__er_as_dyn(self.0))
            }
        }
        impl<'a, T: #as_dyn + ?::core::marker::Sized> #as_source<'a> for &#source_ref<'a, &T> {
            fn __er_as_source(self) -> #source_ref<'a, dyn ::core::error::Error> {
                #source_ref(#as_dyn::__er_as_dyn(&**self.0))
            }
        }
        impl<'a, T: #as_dyn + ?::core::marker::Sized> #as_source<'a> for &#source_ref<'a, &mut T> {
            fn __er_as_source(self) -> #source_ref<'a, dyn ::core::error::Error> {
                #source_ref(#as_dyn::__er_as_dyn(&**self.0))
            }
        }
        #(#pointers)*
    }
}
