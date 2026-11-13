use crate::input::{Field, Input};
use proc_macro2::{Span, TokenStream, TokenTree};
use quote::{ToTokens as _, quote_spanned};
use syn::spanned::Spanned as _;

const TREES: [&str; 2] = ["ErTree", "ErSharedTree"];

pub fn warnings(input: &Input<'_>) -> TokenStream {
    if !TREES.iter().any(|name| input.names.contains(*name)) || allows_deprecated(&input.item.attrs)
    {
        return TokenStream::new();
    }

    let mut warnings = TokenStream::new();
    let fields = input
        .cases
        .iter()
        .filter(|case| !allows_deprecated(case.attrs))
        .flat_map(|case| &case.fields);
    for field in fields {
        if !allows_deprecated(&field.item.attrs) && holds_tree(field.item.ty.to_token_stream()) {
            warnings.extend(quote_spanned! {span(field)=>
                const _: () = {
                    #[deprecated(note = "errors go in the tree, not in a field. Put it below your error with `.er()`, use `.er_share()` when several errors need it")]
                    const fn er_tree_in_a_field() {}
                    er_tree_in_a_field()
                };
            });
        }
    }

    warnings
}

fn span(field: &Field<'_>) -> Span {
    match &field.item.ident {
        Some(name) => name.span(),
        None => field.item.ty.span(),
    }
}

fn allows_deprecated(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|attr| {
        attr.path().is_ident("allow")
            && attr
                .meta
                .to_token_stream()
                .into_iter()
                .any(|token| holds_ident(token, &["deprecated"]))
    })
}

fn holds_tree(tokens: TokenStream) -> bool {
    tokens.into_iter().any(|token| holds_ident(token, &TREES))
}

fn holds_ident(token: TokenTree, names: &[&str]) -> bool {
    match token {
        TokenTree::Ident(ident) => names.iter().any(|name| ident == name),
        TokenTree::Group(group) => group
            .stream()
            .into_iter()
            .any(|token| holds_ident(token, names)),
        _ => false,
    }
}
