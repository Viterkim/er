use crate::{
    er_all,
    names::{binding, identifiers, shield_bindings},
};
use proc_macro2::{TokenStream, TokenTree};
use quote::{format_ident, quote, quote_spanned};
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
    let mut hash = DefaultHasher::new();
    tokens.to_string().hash(&mut hash);
    let suffix = hash.finish();
    let ok = binding(&reserved, &format!("__er_try_values_{suffix:x}"));
    let errors = binding(&reserved, &format!("__er_try_errors_{suffix:x}"));

    let items = results.iter().map(|result| {
        let span = er_all::expression_span(result);
        quote_spanned!(span=> #er_path::internal::ErMacroItem(#result, ::core::marker::PhantomData).item(0))
    });
    let gathered = if results.len() <= 12 {
        let helper = format_ident!("try_all_{}", results.len());
        quote!(#er_path::aggregate::#helper(#(#items),*))
    } else {
        let values: Vec<_> = (0..results.len())
            .map(|index| binding(&reserved, &format!("__er_value_{index}")))
            .collect();
        let shields = shield_bindings(&values);
        let types = values
            .iter()
            .map(|_| quote!(::core::result::Result<_, #er_path::ErPart>));

        quote! {
            ({
                #shields
                #[allow(clippy::result_large_err)]
                |(#(#values,)*): (#(#types,)*)| match (#(#values,)*) {
                    (#(::core::result::Result::Ok(#values),)*) =>
                        ::core::result::Result::Ok((#(#values,)*)),
                    (#(#values,)*) => ::core::result::Result::Err(
                        #er_path::aggregate::gather_parts(
                            &mut [#(#values.map(::core::mem::drop)),*],
                        ),
                    ),
                }
            })((#(#items,)*))
        }
    };

    quote! {{
        #[allow(unused_imports)]
        use #er_path::internal::ErWrappedFallback as _;

        match #gathered {
            ::core::result::Result::Ok(#ok) => ::core::result::Result::Ok(#ok),
            ::core::result::Result::Err(#errors) => ::core::result::Result::Err(
                #er_path::aggregate::CollectedErrors::into_tree(
                    #errors,
                    #er_path::aggregate::ErBatchTop::into_batch_top(#top),
                ),
            ),
        }
    }}
}
