use crate::names::{binding, identifiers, shield_bindings};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{Path, Token, parse::ParseStream, parse::Parser};

pub fn expand(tokens: TokenStream) -> syn::Result<TokenStream> {
    let error = binding(&identifiers(tokens.clone()), "__er_assert_error");
    let shield = shield_bindings([&error]);

    let parse = |input: ParseStream<'_>| {
        let path: Path = input.parse()?;
        input.parse::<Token![,]>()?;
        let result: proc_macro2::TokenTree = input.parse()?;
        input.parse::<Token![,]>()?;
        let pattern: TokenStream = input.parse()?;

        Ok(quote! {{
            #shield
            let #error = #path::er_internal::ErAssertInput::test_err(0, #result)?;
            if !::core::matches!(#path::er_internal::ErInput::er_input_error(&#error.0), #pattern) {
                return ::core::result::Result::Err(::core::convert::From::from(
                    #path::ErTestFailure::__er_expected(#error.0, ::core::stringify!(#pattern)),
                ));
            }
        }})
    };

    parse.parse2(tokens)
}
