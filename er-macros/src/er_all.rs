use crate::names::{binding, identifiers, shield_bindings};
use proc_macro2::{Delimiter, Span, TokenStream, TokenTree};
use quote::{quote, quote_spanned};
use syn::{Path, Token, parse::Parse, parse::ParseStream};

pub enum Input {
    Array(TokenStream),
    Batch {
        er_path: Path,
        collected: TokenStream,
        top: TokenTree,
    },
}

impl Parse for Input {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let er_path: Path = input.parse()?;
        input.parse::<Token![,]>()?;
        let collected = if input.peek(syn::token::Bracket) {
            let content;
            syn::bracketed!(content in input);
            let results = content.parse_terminated(TokenTree::parse, Token![,])?;
            let items = results.iter().map(|result| {
                let span = expression_span(result);
                quote_spanned!(span=>
                    <_ as #er_path::internal::ErAggregateItem<_, _>>::item(0, #result)
                        .map(::core::mem::drop)
                )
            });
            let array = quote!([#(#items),*]);
            if input.is_empty() {
                return Ok(Self::Array(array));
            }

            quote!(#er_path::aggregate::gather_parts(&mut #array))
        } else {
            let expr: TokenTree = input.parse()?;
            quote!(#expr)
        };

        input.parse::<Token![,]>()?;
        Ok(Self::Batch {
            er_path,
            collected,
            top: input.parse()?,
        })
    }
}

pub fn expand(input: Input) -> TokenStream {
    let (er_path, collected, top) = match input {
        Input::Array(array) => return array,
        Input::Batch {
            er_path,
            collected,
            top,
        } => (er_path, collected, top),
    };
    let errors = binding(&identifiers(quote!(#top #collected)), "__er_batch");
    let shield = shield_bindings([&errors]);

    quote! {{
        #shield
        let #errors = #collected;
        if #errors.is_empty() {
            ::core::result::Result::Ok(())
        } else {
            ::core::result::Result::Err(#errors.into_tree(
                #er_path::aggregate::ErBatchTop::into_batch_top(#top)
            ))
        }
    }}
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
