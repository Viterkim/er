use crate::names::{binding, identifiers, shield_bindings};
use proc_macro2::{Delimiter, Span, TokenStream, TokenTree};
use quote::{quote, quote_spanned};
use syn::{Path, Token, parse::Parse, parse::ParseStream};

pub struct Input {
    pub er_path: Path,
    pub results: Vec<TokenTree>,
    pub top: Option<TokenTree>,
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
        let top = if input.peek(Token![,]) {
            input.parse::<Token![,]>()?;
            Some(input.parse()?)
        } else {
            None
        };

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
    let trait_alias = syn::Ident::new("__ErAllItem", Span::call_site());

    let items = results.iter().map(|result| {
        let span = expression_span(result);
        let mut call_alias = trait_alias.clone();
        call_alias.set_span(span);
        quote_spanned! {span=>
            ({
                use #er_path::ErAllItem as #trait_alias;
                #call_alias::er_all_item
            })(#result).map(::core::mem::drop)
        }
    });
    let array = quote!([#(#items),*]);
    match top {
        None => array,
        Some(top) => {
            let reserved = identifiers(quote!(#top #(#results)*));
            let results = binding(&reserved, "__er_results");
            let make = binding(&reserved, "__er_make");
            let shields = shield_bindings([&results, &make]);

            quote! {
                ({
                    #shields

                    |#results: [::core::result::Result<(), #er_path::ErPart>; _], #make|
                        #er_path::aggregate::collect(#make, #results)
                })(#array, || #er_path::ErMake::er_make(#top))
            }
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
