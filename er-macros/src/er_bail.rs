use crate::names::{binding, identifiers, shield_bindings};
use proc_macro2::{TokenStream, TokenTree};
use quote::quote;
use syn::{Path, Token, parse::ParseStream, parse::Parser};

pub fn expand(tokens: TokenStream) -> syn::Result<TokenStream> {
    let reserved = identifiers(tokens.clone());
    let target = binding(&reserved, "__er_bail_target");
    let tree = binding(&reserved, "__er_bail_tree");
    let shield = shield_bindings([&target, &tree]);

    let parse = |input: ParseStream<'_>| {
        let path: Path = input.parse()?;
        input.parse::<Token![,]>()?;
        let root: TokenTree = input.parse()?;
        input.parse::<Token![,]>()?;
        let nodes: TokenStream = input.parse()?;

        Ok(quote! {{
            #shield
            #[allow(unused_imports)]
            use #path::er_internal::ErNewPass as _;

            let #tree = #path::er_internal::ErNew(#root)
                .__er_tree()
                .__er_factory()
                .__er_tree()
                .__er_bail_root();
            let (#target, #tree) = #path::er_internal::ErBailNodes::prepare(0, #tree);

            return ::core::result::Result::Err(
                #path::er_internal::ErBailTree::finish(#path::er_add!(#tree, #nodes), #target),
            );
        }})
    };

    parse.parse2(tokens)
}
