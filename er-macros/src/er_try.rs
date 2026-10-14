use crate::{
    er_all::expression_span,
    names::{binding, identifiers, shield_bindings},
};
use proc_macro2::{TokenStream, TokenTree};
use quote::{quote, quote_spanned};
use std::hash::{DefaultHasher, Hash, Hasher};
use syn::{Path, Token, parse::Parse, parse::ParseStream};

pub struct Input {
    pub er_path: Path,
    pub results: Vec<TokenTree>,
    pub top: TokenTree,
}
impl Parse for Input {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let er_path = input.parse()?;
        input.parse::<Token![,]>()?;
        let content;
        syn::bracketed!(content in input);
        let results = content
            .parse_terminated(TokenTree::parse, Token![,])?
            .into_iter()
            .collect();
        input.parse::<Token![,]>()?;
        let top = input.parse()?;

        Ok(Self {
            er_path,
            results,
            top,
        })
    }
}

pub fn expand(
    Input {
        er_path,
        results,
        top,
    }: Input,
) -> TokenStream {
    let tokens = quote!(#top #(#results)*);
    let reserved = identifiers(tokens.clone());
    let values: Vec<_> = (0..results.len())
        .map(|index| binding(&reserved, &format!("__er_value_{index}")))
        .collect();
    let shields = shield_bindings(&values);
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

    let types = values
        .iter()
        .map(|_| quote!(::core::result::Result<_, #er_path::ErPart>));

    let gathered = quote! {
        ({
            #shields
            #[allow(clippy::result_large_err)]
            |(#(#values,)*): (#(#types,)*)| match (#(#values,)*) {
                (#(::core::result::Result::Ok(#values),)*) =>
                    ::core::result::Result::Ok((#(#values,)*)),
                (#(#values,)*) =>
                    ::core::result::Result::Err([#(::core::result::Result::err(#values)),*]),
            }
        })((#(#items,)*))
    };

    // Imports around the match would hide constants inside caller macros.
    let mut hash = DefaultHasher::new();
    tokens.to_string().hash(&mut hash);
    let suffix = hash.finish();
    let values = binding(&reserved, &format!("__er_try_values_{suffix:x}"));
    let errors = binding(&reserved, &format!("__er_try_errors_{suffix:x}"));

    quote! {
        match #gathered {
            ::core::result::Result::Ok(#values) => ::core::result::Result::Ok(#values),
            ::core::result::Result::Err(#errors) => ::core::result::Result::Err(
                #er_path::ErTree::new(
                    #er_path::ErMake::er_make(#top),
                    ::core::iter::Iterator::flatten(::core::iter::IntoIterator::into_iter(#errors)),
                ),
            ),
        }
    }
}
