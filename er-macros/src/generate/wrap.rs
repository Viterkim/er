use super::replace_self::ReplaceSelf;
use crate::input::attrs::{WrapOptions, WrapOutput};
use crate::names::{binding, identifiers, shield_bindings};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use std::collections::HashSet;
use syn::fold::Fold as _;
use syn::{DeriveInput, Generics, Ident, Lifetime, Path, Type, parse_quote};

pub fn expand(
    input: &DeriveInput,
    er_path: &Path,
    options: &WrapOptions,
    reserved: &HashSet<String>,
    formatter: &Ident,
) -> TokenStream {
    let tree = binding(reserved, "tree");
    let value = binding(reserved, "error");
    let report = binding(reserved, "report");
    let top = binding(reserved, "top");
    let wrapped = binding(reserved, "wrapped");
    let shields = shield_bindings([&tree, &value, &report, &top, &wrapped]);

    let error = &input.ident;
    let wrap = match &options.name {
        Some(name) => name.clone(),
        None => format_ident!("{}Wrap", error),
    };
    let visibility = &input.vis;

    let (_, type_generics, _) = input.generics.split_for_impl();
    let stored: Type = parse_quote!(#error #type_generics);
    let mut replace_self = ReplaceSelf { original: &stored };
    let declaration = replace_self.fold_generics(input.generics.clone());
    let (impl_generics, type_generics, where_clause) = declaration.split_for_impl();
    let inner = quote!(#er_path::ErTree<#stored>);

    let mut root_generics = declaration.clone();
    root_generics
        .make_where_clause()
        .predicates
        .push(parse_quote!(#stored: ::core::error::Error + 'static));
    let (root_impl, _, root_where) = root_generics.split_for_impl();

    let mut top_generics = declaration.clone();
    top_generics
        .make_where_clause()
        .predicates
        .push(parse_quote!(#stored: ::core::fmt::Display));
    let (top_impl, _, top_where) = top_generics.split_for_impl();

    // Don't shadow names from the user's bounds, including trait names.
    let node_names = identifiers(quote!(#declaration #where_clause #stored #wrap #er_path));
    let node_root = binding(&node_names, "__ErNodeRoot");
    let mode = binding(&node_names, "__ErContextMode");
    let new_top = binding(&node_names, "__ErNewTop");
    let alloc = binding(&node_names, "__er_alloc");
    let mut node_generics = declaration.clone();
    node_generics.params.push(parse_quote!(#node_root));
    // The runtime decides which errors can become stored nodes.
    let predicates = &mut node_generics.make_where_clause().predicates;
    predicates.push(parse_quote!(
        #wrap #type_generics: #er_path::IntoErTree<Error = #node_root>
    ));
    predicates.push(parse_quote!(
        #node_root: ::core::error::Error + ::core::convert::Into<#er_path::BoxError> + 'static
    ));
    let (node_impl, _, node_where) = node_generics.split_for_impl();

    let conversion = if options.std_error {
        quote! {
            impl #root_impl ::core::error::Error for #wrap #type_generics #root_where {}
        }
    } else {
        let mut context_generics = node_generics.clone();
        context_generics.params.push(parse_quote!(#mode));
        let (context_impl, _, context_where) = context_generics.split_for_impl();

        quote! {
            impl #node_impl #er_path::IntoErPart for #wrap #type_generics #node_where {
                type Error = #stored;

                fn er_error(&self) -> &#stored {
                    &self.tree.top
                }

                fn into_er_part(self) -> #er_path::ErPart {
                    let #tree: #er_path::ErTree<#node_root> =
                        #er_path::IntoErTree::into_er_tree(self);
                    #er_path::IntoErPart::into_er_part(#tree)
                }
            }
            impl #node_impl #er_path::IntoErPart for #alloc::boxed::Box<#wrap #type_generics> #node_where {
                type Error = #stored;

                fn er_error(&self) -> &#stored {
                    &self.tree.top
                }

                #[track_caller]
                fn into_er_part(self) -> #er_path::ErPart {
                    #er_path::IntoErPart::into_er_part(*self)
                }
            }
            impl #context_impl #er_path::ErTreeContextExt<#mode> for #wrap #type_generics #context_where {
                #[track_caller]
                fn er<#new_top>(self, #top: impl #er_path::ErMake<#new_top, #mode>) -> #er_path::ErTree<#new_top>
                where
                    #new_top: ::core::error::Error + 'static,
                {
                    #er_path::IntoErTree::er_wrap(self, #top)
                }
            }
        }
    };

    let formatting = match options.output {
        None => TokenStream::new(),
        Some(output) => {
            let (generics, view) = match output {
                WrapOutput::Top => (&top_generics, format_ident!("er_top")),
                WrapOutput::Report => (&root_generics, format_ident!("er_report")),
            };

            formatting(er_path, &wrap, generics, &view, formatter)
        }
    };

    let std_error_docs = options.std_error.then(|| {
        quote! {
            /// ! WARNING ! `.er()` and `.er_with()` on a Result with this hide the sub errors from find!
            /// Use `.er_wrap()`, or `.er_with_wrap()` with the old top. I can't enforce it, sorry.
        }
    });

    let presentation = options.std_error.then(|| {
        quote! {
            const _: () = {
                impl #root_impl #wrap #type_generics #root_where {
                    /// The whole report as text.
                    pub fn er_report_string(&self) -> #alloc::string::String {
                        self.tree.er_report_string()
                    }

                    /// Save the messages and tree structure.
                    pub fn er_snapshot(&self) -> #er_path::ErSnapshot {
                        self.tree.er_snapshot()
                    }
                }

                impl #top_impl #wrap #type_generics #top_where {
                    /// Just the outer error as text.
                    pub fn er_top_string(&self) -> #alloc::string::String {
                        self.tree.er_top_string()
                    }
                }
            };
        }
    });

    quote! {
        /// Your tree, in a type you can implement foreign traits on.
        #std_error_docs
        #[must_use]
        #visibility struct #wrap #declaration #where_clause {
            pub tree: #inner,
        }
        const _: () = {
            #shields
            extern crate alloc as #alloc;

            impl #impl_generics ::core::ops::Deref for #wrap #type_generics #where_clause {
                type Target = #inner;

                fn deref(&self) -> &Self::Target {
                    &self.tree
                }
            }
            impl #impl_generics ::core::convert::AsMut<#inner> for #wrap #type_generics #where_clause {
                fn as_mut(&mut self) -> &mut #inner {
                    &mut self.tree
                }
            }
            impl #impl_generics ::core::convert::AsMut<#inner> for #alloc::boxed::Box<#wrap #type_generics> #where_clause {
                fn as_mut(&mut self) -> &mut #inner {
                    &mut self.tree
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
            impl #node_impl #wrap #type_generics #node_where {
                /// Build the new top from fields taken from the old top.
                #[track_caller]
                pub fn er_with<#new_top, #mode>(self, #top: impl #er_path::ErWithFields<#node_root, #new_top, #mode>) -> #er_path::ErTree<#new_top>
                where
                    #new_top: ::core::error::Error + 'static,
                {
                    #er_path::IntoErTree::er_with(self, #top)
                }
            }
            impl #root_impl ::core::convert::From<#stored> for #wrap #type_generics #root_where {
                #[track_caller]
                fn from(#value: #stored) -> Self {
                    let #tree = #er_path::ErTree::from(#value);
                    Self { tree: #tree }
                }
            }
            #presentation
            impl #root_impl #stored #root_where {
                /// Put this error in its Wrap.
                #[track_caller]
                pub fn er_wrap(self) -> #wrap #type_generics {
                    ::core::convert::From::from(self)
                }
            }
            #formatting
        };
    }
}

fn formatting(
    er_path: &Path,
    wrap: &Ident,
    generics: &Generics,
    view: &Ident,
    formatter: &Ident,
) -> TokenStream {
    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();
    let reserved = identifiers(quote!(#generics #wrap #er_path));
    let failure = binding(&reserved, "__ErLineFailure");
    let emit = binding(&reserved, "__er_emit");
    let shields = shield_bindings([formatter, &emit]);
    let borrow = Lifetime::new(
        &format!("'{}", binding(&reserved, "__er_view")),
        formatter.span(),
    );
    let mut borrowed_generics = generics.clone();
    borrowed_generics.params.insert(0, parse_quote!(#borrow));
    let (borrowed_impl, _, borrowed_where) = borrowed_generics.split_for_impl();

    quote! {
        const _: () = {
            #shields

            impl #impl_generics #wrap #type_generics #where_clause {
                /// Only formats once, no line endings.
                pub fn for_each_line(&self, #emit: impl ::core::ops::FnMut(&str)) -> ::core::fmt::Result {
                    #er_path::lines::for_each_line(self, #emit)
                }

                /// Like `for_each_line()`, but the callback can fail.
                pub fn try_for_each_line<#failure>(
                    &self,
                    #emit: impl ::core::ops::FnMut(&str) -> ::core::result::Result<(), #failure>,
                ) -> ::core::result::Result<(), #er_path::ErLineError<#failure>> {
                    #er_path::lines::try_for_each_line(self, #emit)
                }
            }
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
            impl #impl_generics #er_path::ErOpaqueErrorExt for #wrap #type_generics #where_clause {
                type Output = #er_path::ErAsError<Self>;

                fn opaque_err(self) -> Self::Output {
                    #er_path::ErAsError(self)
                }
            }
            impl #borrowed_impl #er_path::ErOpaqueErrorExt for &#borrow #wrap #type_generics #borrowed_where {
                type Output = #er_path::ErAsError<Self>;

                fn opaque_err(self) -> Self::Output {
                    #er_path::ErAsError(self)
                }
            }
        };
    }
}
