use crate::{er_all::expression_span, names::binding};
use proc_macro2::{TokenStream, TokenTree};
use quote::{quote, quote_spanned};
use std::collections::HashSet;
use syn::{Path, Token, parse::Parse, parse::ParseStream};

pub struct Input {
    pub er_path: Path,
    pub top: TokenTree,
    pub results: Vec<TokenTree>,
}
impl Parse for Input {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let er_path = input.parse()?;
        input.parse::<Token![,]>()?;
        let top = input.parse()?;
        input.parse::<Token![,]>()?;
        let content;
        syn::bracketed!(content in input);
        let mut results = Vec::new();
        while !content.is_empty() {
            results.push(content.parse()?);
            if !content.is_empty() {
                content.parse::<Token![,]>()?;
            }
        }

        Ok(Self {
            er_path,
            top,
            results,
        })
    }
}

pub fn expand(
    Input {
        er_path,
        top,
        results,
    }: Input,
) -> TokenStream {
    let reserved = HashSet::new();
    let values: Vec<_> = (0..results.len())
        .map(|index| binding(&reserved, &format!("__er_value_{index}")))
        .collect();
    let errors = binding(&reserved, "__er_errors");
    let item = binding(&reserved, "__ErTryItem");
    let items = results.iter().map(|result| {
        let span = expression_span(result);
        let mut call = item.clone();
        call.set_span(span);
        quote_spanned! {span=>
            ({
                use #er_path::ErAllItem as #item;
                #call::er_all_item
            })(#result)
        }
    });

    quote! {
        match (#(#items,)*) {
            (#(::core::result::Result::Ok(#values),)*) =>
                ::core::result::Result::Ok((#(#values,)*)),
            (#(#values,)*) => {
                let #errors = [#(::core::result::Result::err(#values)),*];
                ::core::result::Result::Err(#er_path::ErTree::new(
                    #er_path::ErMake::er_make(#top),
                    ::core::iter::Iterator::flatten(::core::iter::IntoIterator::into_iter(#errors)),
                ))
            }
        }
    }
}
