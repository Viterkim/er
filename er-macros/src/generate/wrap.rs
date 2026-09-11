use super::replace_self::ReplaceSelf;
use crate::input::attrs::{WrapOptions, WrapOutput};
use crate::names::binding;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use std::collections::HashSet;
use syn::fold::Fold as _;
use syn::{DeriveInput, Generics, Ident, Path, Type};

pub fn expand(
    input: &DeriveInput,
    er_path: &Path,
    options: &WrapOptions,
    reserved: &HashSet<String>,
    formatter: &Ident,
) -> syn::Result<TokenStream> {
    let tree = binding(reserved, "tree");
    let value = binding(reserved, "error");
    let report = binding(reserved, "report");
    let top = binding(reserved, "top");
    let wrapped = binding(reserved, "wrapped");

    let error = &input.ident;
    let wrap = match &options.name {
        Some(name) => name.clone(),
        None => format_ident!("{}Wrap", error),
    };
    let visibility = &input.vis;

    let (_, type_generics, _) = input.generics.split_for_impl();
    let stored = syn::parse2(quote!(#error #type_generics))?;
    let mut replace_self = ReplaceSelf { original: &stored };
    let declaration = replace_self.fold_generics(input.generics.clone());
    let (impl_generics, type_generics, where_clause) = declaration.split_for_impl();
    let inner = quote!(#er_path::ErTree<#stored>);

    let mut root_generics = declaration.clone();
    let root_bound = syn::parse2(quote!(
        #stored: ::core::error::Error + 'static
    ))?;
    root_generics
        .make_where_clause()
        .predicates
        .push(root_bound);
    let (root_impl, _, root_where) = root_generics.split_for_impl();

    let formatting = match options.output {
        None => TokenStream::new(),
        Some(output) => formatting(
            &wrap,
            &declaration,
            &root_generics,
            &stored,
            output,
            formatter,
        )?,
    };

    Ok(quote! {
        #[must_use]
        #visibility struct #wrap #declaration #where_clause {
            pub tree: #inner,
        }
        impl #impl_generics ::core::ops::Deref for #wrap #type_generics #where_clause {
            type Target = #inner;

            fn deref(&self) -> &Self::Target {
                &self.tree
            }
        }
        impl #impl_generics ::core::convert::From<#inner> for #wrap #type_generics #where_clause {
            fn from(#tree: #inner) -> Self {
                Self { tree: #tree }
            }
        }
        impl #impl_generics ::core::convert::From<#er_path::ErReport<#stored>> for #wrap #type_generics #where_clause {
            fn from(#report: #er_path::ErReport<#stored>) -> Self {
                Self { tree: #report.tree }
            }
        }
        impl #impl_generics ::core::convert::From<#er_path::ErTop<#stored>> for #wrap #type_generics #where_clause {
            fn from(#top: #er_path::ErTop<#stored>) -> Self {
                Self { tree: #top.tree }
            }
        }
        impl #impl_generics ::core::convert::From<#wrap #type_generics> for #inner #where_clause {
            fn from(#wrapped: #wrap #type_generics) -> Self {
                #wrapped.tree
            }
        }
        impl #impl_generics ::core::convert::From<#wrap #type_generics> for #er_path::ErReport<#stored> #where_clause {
            fn from(#wrapped: #wrap #type_generics) -> Self {
                #wrapped.tree.into_er_report()
            }
        }
        impl #impl_generics ::core::convert::From<#wrap #type_generics> for #er_path::ErTop<#stored> #where_clause {
            fn from(#wrapped: #wrap #type_generics) -> Self {
                #wrapped.tree.into_er_top()
            }
        }
        impl #impl_generics #er_path::IntoErTree for #wrap #type_generics #where_clause {
            type Error = #stored;

            fn into_er_tree(self) -> #inner {
                self.tree
            }
        }
        impl #root_impl ::core::convert::From<#stored> for #wrap #type_generics #root_where {
            #[track_caller]
            fn from(#value: #stored) -> Self {
                let #tree = #er_path::ErTree::from(#value);
                Self { tree: #tree }
            }
        }
        impl #root_impl #stored #root_where {
            #[track_caller]
            pub fn er_wrap(self) -> #wrap #type_generics {
                ::core::convert::From::from(self)
            }
        }
        #formatting
    })
}

pub fn formatting(
    wrap: &Ident,
    declaration: &Generics,
    error_generics: &Generics,
    stored: &Type,
    output: WrapOutput,
    formatter: &Ident,
) -> syn::Result<TokenStream> {
    let (view, format_generics) = match output {
        WrapOutput::Top => {
            let mut generics = declaration.clone();
            let bound = syn::parse2(quote!(#stored: ::core::fmt::Display))?;
            generics.make_where_clause().predicates.push(bound);
            (quote!(er_top), generics)
        }
        WrapOutput::Report => (quote!(er_report), error_generics.clone()),
    };
    let (impl_generics, type_generics, where_clause) = format_generics.split_for_impl();
    let (error_impl, _, error_where) = error_generics.split_for_impl();

    Ok(quote! {
        impl #impl_generics ::core::fmt::Display for #wrap #type_generics #where_clause {
            fn fmt(&self, #formatter: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                ::core::fmt::Display::fmt(&self.tree.#view(), #formatter)
            }
        }
        impl #impl_generics ::core::fmt::Debug for #wrap #type_generics #where_clause {
            fn fmt(&self, #formatter: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                ::core::fmt::Display::fmt(&self.tree.#view(), #formatter)
            }
        }
        impl #error_impl ::core::error::Error for #wrap #type_generics #error_where {}
    })
}
