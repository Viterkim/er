use super::replace_self::ReplaceSelf;
use crate::input::attrs::{WrapOptions, WrapOutput};
use crate::names::binding;
use proc_macro2::{TokenStream, TokenTree};
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

    let conversion = if options.std_error {
        quote! {
            impl #root_impl ::core::error::Error for #wrap #type_generics #root_where {}
        }
    } else {
        // Don't shadow names from the user's bounds, including trait names.
        let mut node_names = HashSet::new();
        let mut tokens: Vec<_> = quote!(#declaration #where_clause #stored #wrap #er_path)
            .into_iter()
            .collect();
        while let Some(token) = tokens.pop() {
            match token {
                TokenTree::Ident(ident) => {
                    node_names.insert(crate::names::plain(&ident));
                }
                TokenTree::Group(group) => tokens.extend(group.stream()),
                _ => {}
            }
        }
        let node_root = binding(&node_names, "__ErNodeRoot");
        let mut node_generics = declaration.clone();
        node_generics.params.push(syn::parse2(quote!(#node_root))?);
        // A Wrap containing Rc must still compile. Only turning it into a node needs Send/Sync.
        let predicates = &mut node_generics.make_where_clause().predicates;
        predicates.push(syn::parse2(quote!(
            #wrap #type_generics: #er_path::IntoErTree<Error = #node_root>
        ))?);
        predicates.push(syn::parse2(quote!(
            #node_root: ::core::error::Error + ::core::marker::Send + ::core::marker::Sync + 'static
        ))?);
        let (node_impl, _, node_where) = node_generics.split_for_impl();

        quote! {
            impl #node_impl #er_path::IntoErNode for #wrap #type_generics #node_where {
                fn into_er_node(self) -> #er_path::ErNode {
                    let #tree: #er_path::ErTree<#node_root> =
                        #er_path::IntoErTree::into_er_tree(self);
                    #er_path::IntoErNode::into_er_node(#tree)
                }
            }
        }
    };

    let formatting = match options.output {
        None => TokenStream::new(),
        Some(output) => formatting(
            er_path,
            &wrap,
            &declaration,
            &root_generics,
            &stored,
            output,
            formatter,
        )?,
    };

    let std_error_docs = options.std_error.then(|| {
        quote! {
            /// **WARNING: This is a piece of shit. With this `.er(...)` on the Result hides the sub errors from find!**
            /// **WARNING You HAVE to use `.er_from_wrap(||)` instead of `.er()` and there's no way i can help you enforce it, I'm sorry...**
        }
    });

    Ok(quote! {
        #std_error_docs
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
        #conversion
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

fn formatting(
    er_path: &Path,
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

    Ok(quote! {
        impl #impl_generics ::core::fmt::Display for #wrap #type_generics #where_clause {
            fn fmt(&self, #formatter: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                ::core::fmt::Display::fmt(&self.tree.#view(), #formatter)
            }
        }
        impl #impl_generics ::core::fmt::Debug for #wrap #type_generics #where_clause {
            fn fmt(&self, #formatter: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                ::core::fmt::Display::fmt(self, #formatter)
            }
        }
        impl #impl_generics #er_path::ErOpaqueError for #wrap #type_generics #where_clause {
            type Output = #er_path::ErAsError<Self>;

            fn opaque_err(self) -> Self::Output {
                #er_path::ErAsError(self)
            }
        }
    })
}
