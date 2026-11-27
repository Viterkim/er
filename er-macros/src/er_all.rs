use crate::names::{binding, identifiers, shield_bindings};
use proc_macro2::{Delimiter, Span, TokenStream, TokenTree};
use quote::{quote, quote_spanned};
use syn::{Path, Token, parse::Parse, parse::ParseStream};

pub enum Input {
    Add(TokenStream),
    Batch {
        er_path: Path,
        collected: BatchItems,
        top: TokenTree,
    },
}

pub enum BatchItems {
    List(Vec<TokenStream>),
    Gathered(TokenTree),
}

impl Parse for Input {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let er_path: Path = input.parse()?;
        input.parse::<Token![,]>()?;
        let collected = if input.peek(syn::token::Bracket) {
            let content;
            syn::bracketed!(content in input);
            let results = content.parse_terminated(TokenTree::parse, Token![,])?;
            if input.peek(Token![;]) {
                input.parse::<Token![;]>()?;
                let top: TokenTree = input.parse()?;
                if results.len() == 1 {
                    let only = &results[0];
                    let root = quote!(#er_path::__er_add_root!(#top));
                    // The parentheses carry the child's span through track_caller.
                    return Ok(Self::Add(quote_spanned! {expression_span(only)=>
                        (#er_path::er_internal::aggregate::add_one)(#root, 0, #only)
                    }));
                }

                let items = results.iter().map(|result| item(&er_path, result));

                return Ok(Self::Add(quote! {
                    #er_path::er_internal::aggregate::add_nodes(
                        #er_path::__er_add_root!(#top),
                        &mut [#(#items),*],
                    )
                }));
            }

            BatchItems::List(
                results
                    .iter()
                    .map(|result| item(&er_path, result))
                    .collect(),
            )
        } else {
            let expr: TokenTree = input.parse()?;
            BatchItems::Gathered(expr)
        };

        input.parse::<Token![,]>()?;
        Ok(Self::Batch {
            er_path,
            collected,
            top: input.parse()?,
        })
    }
}

fn item(er_path: &Path, result: &TokenTree) -> TokenStream {
    quote_spanned!(expression_span(result)=>
        <_ as #er_path::er_internal::ErAggregateItem<_, _>>::item(0, #result)
            .map(::core::mem::drop)
    )
}

pub fn expand(input: Input) -> TokenStream {
    let (er_path, collected, top) = match input {
        Input::Add(tree) => return tree,
        Input::Batch {
            er_path,
            collected,
            top,
        } => (er_path, collected, top),
    };
    let top = quote!(#er_path::er_internal::aggregate::ErBatchTop::into_batch_top(#top));

    match collected {
        // Building the array here would copy every item even when they all succeeded.
        BatchItems::List(items) => {
            let reserved = identifiers(quote!(#top #(#items)*));
            let names: Vec<_> = (0..items.len())
                .map(|index| binding(&reserved, &format!("__er_item_{index}")))
                .collect();
            let shield = shield_bindings(&names);

            quote! {{
                #shield
                #(let #names = #items;)*
                if true #(&& ::core::result::Result::is_ok(&#names))* {
                    ::core::result::Result::Ok(())
                } else {
                    ::core::result::Result::Err(
                        #er_path::er_internal::aggregate::tree_from_nodes(
                            #er_path::er_internal::aggregate::gather_nodes(&mut [#(#names),*]),
                            #top,
                        ),
                    )
                }
            }}
        }
        BatchItems::Gathered(collected) => {
            let errors = binding(&identifiers(quote!(#top #collected)), "__er_batch");
            let shield = shield_bindings([&errors]);

            quote! {{
                #shield
                let #errors = #collected;
                if #errors.is_empty() {
                    ::core::result::Result::Ok(())
                } else {
                    ::core::result::Result::Err(#er_path::er_internal::aggregate::tree_from_nodes(#errors, #top))
                }
            }}
        }
    }
}

pub fn expression_span(result: &TokenTree) -> Span {
    match result {
        TokenTree::Group(group) if group.delimiter() == Delimiter::None => {
            // macro_rules! gives us a group whose span points at the macro invocation.
            group
                .stream()
                .into_iter()
                .next()
                .map_or(group.span(), |first| expression_span(&first))
        }
        _ => result.span(),
    }
}
