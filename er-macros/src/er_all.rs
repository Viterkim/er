use proc_macro2::{Delimiter, Span, TokenStream, TokenTree};
use quote::{quote, quote_spanned};
use syn::{Path, Token, parse::Parse, parse::ParseStream};

pub struct Input {
    pub er_all_item: Path,
    pub results: Vec<TokenTree>,
}
impl Parse for Input {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let er_all_item = input.parse()?;
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
            er_all_item,
            results,
        })
    }
}

pub fn expand(
    Input {
        er_all_item,
        results,
    }: Input,
) -> TokenStream {
    let trait_alias = syn::Ident::new("__ErAllItem", Span::call_site());

    let items = results.iter().map(|result| {
        let span = expression_span(result);
        let mut call_alias = trait_alias.clone();
        call_alias.set_span(span);
        quote_spanned! {span=>
            ({
                use #er_all_item as #trait_alias;
                #call_alias::er_all_item
            })(#result)
        }
    });
    quote!([#(#items),*])
}

fn expression_span(result: &TokenTree) -> Span {
    match result {
        TokenTree::Group(group) if group.delimiter() == Delimiter::None => {
            // macro_rules! gives us a group whose span points at er_all!.
            group
                .stream()
                .into_iter()
                .next()
                .map_or(group.span(), |first| expression_span(&first))
        }
        _ => result.span(),
    }
}
