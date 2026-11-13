use proc_macro2::{Span, TokenStream, TokenTree};
use quote::quote;
use std::collections::HashSet;
use syn::Ident;

pub fn plain(ident: &Ident) -> String {
    let mut name = ident.to_string();
    if name.starts_with("r#") {
        name.drain(..2);
    }
    name
}

pub fn identifiers(tokens: TokenStream) -> HashSet<String> {
    let mut names = HashSet::new();
    let mut pending: Vec<_> = tokens.into_iter().collect();

    while let Some(token) = pending.pop() {
        match token {
            TokenTree::Ident(ident) => {
                names.insert(plain(&ident));
            }
            TokenTree::Group(group) => pending.extend(group.stream()),
            _ => {}
        }
    }

    names
}

pub fn binding(reserved: &HashSet<String>, name: &str) -> Ident {
    if !reserved.contains(name) {
        return Ident::new(name, Span::mixed_site());
    }

    let mut name = name.to_owned();
    while reserved.contains(&name) {
        name.insert(0, '_');
    }
    Ident::new(&name, Span::mixed_site())
}

pub fn shield_bindings<'a>(bindings: impl IntoIterator<Item = &'a Ident>) -> TokenStream {
    let mut seen = HashSet::new();
    let bindings: Vec<_> = bindings
        .into_iter()
        .filter(|binding| seen.insert(plain(binding)))
        .collect();
    if bindings.is_empty() {
        return TokenStream::new();
    }

    // Mixed-site hygiene still resolves constants in patterns. Functions can be shadowed.
    quote! {
        #[allow(unused_imports)]
        use ::core::mem::{#(drop as #bindings),*};
    }
}

pub fn snake_case(name: &str) -> String {
    let name = name.trim_start_matches("r#");
    let mut chars = name.chars().peekable();
    let mut result = String::with_capacity(name.len());
    let mut previous = '_';

    while let Some(ch) = chars.next() {
        if ch.is_uppercase() && previous != '_' {
            let follows_word = previous.is_lowercase() || previous.is_numeric();
            let ends_acronym = chars.peek().is_some_and(|next| next.is_lowercase());
            if follows_word || ends_acronym {
                result.push('_');
            }
        }
        result.extend(ch.to_lowercase());
        previous = ch;
    }

    if matches!(result.as_str(), "self" | "super" | "crate") {
        result.push('_');
    }
    result
}
