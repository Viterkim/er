use super::replace_self::ReplaceSelf;
use crate::input::attrs::{WrapOptions, WrapOutput};
use crate::names::{binding, identifiers, shield_bindings};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use std::collections::HashSet;
use syn::visit_mut::VisitMut as _;
use syn::{DeriveInput, Generics, Ident, Path, Type, parse_quote};

pub fn expand(
    input: &DeriveInput,
    er_path: &Path,
    options: &WrapOptions,
    reserved: &HashSet<String>,
    formatter: &Ident,
) -> TokenStream {
    let tree = binding(reserved, "tree");
    let value = binding(reserved, "error");
    let top = binding(reserved, "top");
    let wrapped = binding(reserved, "wrapped");
    let title = binding(reserved, "title");
    let saved = binding(reserved, "saved");
    let writer = binding(reserved, "writer");
    let shields = shield_bindings([&tree, &value, &top, &wrapped, &title, &saved, &writer]);

    let error = &input.ident;
    let wrap = match &options.name {
        Some(name) => name.clone(),
        None => format_ident!("{}Wrap", error),
    };
    let visibility = &input.vis;

    let (_, type_generics, _) = input.generics.split_for_impl();
    let stored: Type = parse_quote!(#error #type_generics);
    let mut replace_self = ReplaceSelf { original: &stored };
    let mut declaration = input.generics.clone();
    replace_self.visit_generics_mut(&mut declaration);
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

    // Don't shadow names from the user's bounds, including trait names.
    let node_names = identifiers(quote!(#declaration #where_clause #stored #wrap #er_path));
    let node_root = binding(&node_names, "__ErNodeRoot");
    let mode = binding(&node_names, "__ErContextMode");
    let new_top = binding(&node_names, "__ErNewTop");
    let alloc = binding(&node_names, "__er_alloc");
    let mut node_generics = declaration.clone();
    node_generics.params.push(parse_quote!(#node_root));
    let predicates = &mut node_generics.make_where_clause().predicates;
    predicates.push(parse_quote!(
        #wrap #type_generics: #er_path::IntoErTree<Error = #node_root>
    ));
    predicates.push(parse_quote!(
        #node_root: ::core::error::Error + ::core::convert::Into<#er_path::ErBoxError> + 'static
    ));
    let (node_impl, _, node_where) = node_generics.split_for_impl();

    let mut context_generics = node_generics.clone();
    context_generics.params.push(parse_quote!(#mode));
    let (context_impl, _, context_where) = context_generics.split_for_impl();

    let conversion = if options.std_error {
        let item_kind = binding(&node_names, "__ErItemKind");
        quote! {
            impl #root_impl ::core::error::Error for #wrap #type_generics #root_where {}

            impl #impl_generics #wrap #type_generics #where_clause {
                /// Add children to the wrapped tree.
                #[track_caller]
                pub fn er_add<#mode, #item_kind>(self, #value: impl #er_path::er_internal::aggregate::ErAggregateItems<#item_kind, Item: #er_path::er_internal::ErAllItem<#mode>>) -> #inner {
                    self.tree.er_add(#value)
                }
            }
        }
    } else {
        quote! {
            impl #impl_generics #er_path::er_internal::impls::saved::ErSavedError<#er_path::er_internal::ErSavedTree> for #wrap #type_generics #where_clause {
                type Error = #stored;
                type Root = #inner;

                fn with_saved(self, #saved: #er_path::ErSavedList) -> #inner {
                    #er_path::er_internal::impls::saved::ErSavedError::with_saved(self.tree, #saved)
                }
            }
            impl #node_impl #er_path::IntoErNode for #wrap #type_generics #node_where {
                type Error = #stored;

                fn er_error(&self) -> &#stored {
                    &self.tree.top
                }

                fn into_er_node(self) -> #er_path::ErNode {
                    let #tree: #er_path::ErTree<#node_root> =
                        #er_path::IntoErTree::into_er_tree(self);
                    #er_path::IntoErNode::into_er_node(#tree)
                }
            }
            impl #node_impl #er_path::IntoErNode for #alloc::boxed::Box<#wrap #type_generics> #node_where {
                type Error = #stored;

                fn er_error(&self) -> &#stored {
                    &self.tree.top
                }

                #[track_caller]
                fn into_er_node(self) -> #er_path::ErNode {
                    #er_path::IntoErNode::into_er_node(*self)
                }
            }
            impl #context_impl #er_path::ErErrorContextExt<#mode> for #wrap #type_generics #context_where {}
        }
    };

    let formatting = match options.output {
        None => TokenStream::new(),
        Some(output) => {
            let (generics, display) = match output {
                WrapOutput::Top => (
                    &top_generics,
                    quote!(::core::fmt::Display::fmt(&self.tree.top, #formatter)),
                ),
                WrapOutput::Report => (
                    &root_generics,
                    quote!(::core::fmt::Display::fmt(&#er_path::er_internal::InternalReport::new(&self.tree), #formatter)),
                ),
            };

            formatting(er_path, &wrap, generics, &display, formatter)
        }
    };

    let std_error_docs = options.std_error.then(|| {
        quote! {
            /// Use `.er_wrap()` on a Result with this error to keep its tree, `.er_with_wrap()` if you
            /// need the old top. A plain `.er()` or `.er_with()` on that Result boxes the Wrap, so error
            /// searches can't see inside it.
        }
    });

    let std_error_with = options.std_error.then(|| {
        quote! {
            impl #node_impl #wrap #type_generics #node_where {
                /// Build the new top from fields taken from the old top.
                #[track_caller]
                pub fn er_with<#new_top, #mode>(self, #top: impl #er_path::er_internal::ErWithFields<#node_root, #new_top, #mode>) -> #er_path::ErTree<#new_top>
                where
                    #new_top: ::core::error::Error + 'static,
                {
                    #er_path::IntoErTree::er_with(self, #top)
                }
            }
        }
    });

    let presentation = options.std_error.then(|| {
        quote! {
            const _: () = {
                #shields
                impl #root_impl #wrap #type_generics #root_where {
                    /// The whole report as text.
                    pub fn er_report(&self) -> #alloc::string::String {
                        self.tree.er_report()
                    }

                    /// Write the report straight to a buffer. The writer can return `fmt::Error` to stop early.
                    pub fn er_write_report(
                        &self,
                        #writer: &mut (impl ::core::fmt::Write + ?::core::marker::Sized),
                    ) -> ::core::fmt::Result {
                        self.tree.er_write_report(#writer)
                    }

                    /// Save the messages and tree structure.
                    pub fn er_snapshot(&self) -> #er_path::ErSnapshot {
                        self.tree.er_snapshot()
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
            impl #impl_generics ::core::convert::From<#inner> for #wrap #type_generics #where_clause {
                fn from(#tree: #inner) -> Self {
                    Self { tree: #tree }
                }
            }
            impl #impl_generics ::core::convert::From<#er_path::ErBranch<#stored>> for #wrap #type_generics #where_clause {
                #[track_caller]
                fn from(#value: #er_path::ErBranch<#stored>) -> Self {
                    Self { tree: #er_path::IntoErTree::into_er_tree(#value) }
                }
            }
            impl #impl_generics ::core::convert::From<#wrap #type_generics> for #inner #where_clause {
                fn from(#wrapped: #wrap #type_generics) -> Self {
                    #wrapped.tree
                }
            }
            impl #impl_generics #er_path::IntoErTree for #wrap #type_generics #where_clause {
                type Error = #stored;

                fn into_er_tree(self) -> #inner {
                    self.tree
                }
            }
            impl #impl_generics #wrap #type_generics #where_clause {
                /// Save another report on the tree.
                pub fn er_save(self, #title: impl ::core::convert::Into<#alloc::string::String>, #saved: impl #er_path::er_internal::ErSaveInput) -> #inner {
                    self.tree.er_save(#title, #saved)
                }
            }
            #er_path::__er_if_shared! {
                impl #impl_generics #wrap #type_generics #where_clause {
                    /// Share the tree with its typed top and saved reports.
                    #[track_caller]
                    pub fn er_share(self) -> #er_path::ErSharedTree<#stored> {
                        self.tree.er_share()
                    }
                }
            }
            #conversion
            #std_error_with
            impl #root_impl ::core::convert::From<#stored> for #wrap #type_generics #root_where {
                #[track_caller]
                fn from(#value: #stored) -> Self {
                    let #tree = #er_path::ErTree::from(#value);
                    Self { tree: #tree }
                }
            }
            #presentation
            #formatting
        };
    }
}

fn formatting(
    er_path: &Path,
    wrap: &Ident,
    generics: &Generics,
    display: &TokenStream,
    formatter: &Ident,
) -> TokenStream {
    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();
    let reserved = identifiers(quote!(#generics #wrap #er_path));
    let failure = binding(&reserved, "__ErLineFailure");
    let emit = binding(&reserved, "__er_emit");
    let shields = shield_bindings([formatter, &emit]);

    quote! {
        const _: () = {
            #shields

            impl #impl_generics #wrap #type_generics #where_clause {
                /// Formats once and passes each line to the callback without its ending.
                pub fn for_each_line(&self, #emit: impl ::core::ops::FnMut(&::core::primitive::str)) -> ::core::fmt::Result {
                    #er_path::er_internal::lines::for_each_line(self, #emit)
                }

                /// Like `for_each_line()`, but the callback can fail.
                pub fn try_for_each_line<#failure>(
                    &self,
                    #emit: impl ::core::ops::FnMut(&::core::primitive::str) -> ::core::result::Result<(), #failure>,
                ) -> ::core::result::Result<(), #er_path::ErLineError<#failure>> {
                    #er_path::er_internal::lines::try_for_each_line(self, #emit)
                }
            }
            impl #impl_generics ::core::fmt::Display for #wrap #type_generics #where_clause {
                fn fmt(&self, #formatter: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    #display
                }
            }
            impl #impl_generics ::core::fmt::Debug for #wrap #type_generics #where_clause {
                fn fmt(&self, #formatter: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    ::core::fmt::Display::fmt(self, #formatter)
                }
            }


        };
    }
}
