use proc_macro2::{Delimiter, Span, TokenStream, TokenTree};
use quote::{quote, quote_spanned};
use syn::{Path, Token, parse::Parse, parse::ParseStream};

pub struct Input {
    pub into_er_part: Path,
    pub results: Vec<TokenTree>,
}
impl Parse for Input {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let into_er_part = input.parse()?;
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
            into_er_part,
            results,
        })
    }
}

pub fn expand(
    Input {
        into_er_part,
        results,
    }: Input,
) -> TokenStream {
    let trait_alias = syn::Ident::new("__ErAllIntoErPart", Span::call_site());

    let items = results.iter().map(|result| {
        let span = expression_span(result);
        let mut call_alias = trait_alias.clone();
        call_alias.set_span(span);
        quote_spanned! {span=>
            match #result {
                ::core::result::Result::Ok(__er_success) => {
                    let _ = [__er_success];
                    ::core::result::Result::Ok(())
                }
                ::core::result::Result::Err(__er_failure) => {
                    use #into_er_part as #trait_alias;
                    ::core::result::Result::Err(
                        #call_alias::into_er_part(__er_failure)
                    )
                }
            }
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
