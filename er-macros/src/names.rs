use proc_macro2::Span;
use std::collections::HashSet;
use syn::Ident;

pub fn plain(ident: &Ident) -> String {
    let mut name = ident.to_string();
    if name.starts_with("r#") {
        name.drain(..2);
    }
    name
}

pub fn binding(reserved: &HashSet<String>, name: &str) -> Ident {
    let mut name = name.to_owned();
    while reserved.contains(&name) {
        name.insert(0, '_');
    }
    Ident::new(&name, Span::mixed_site())
}

pub fn snake_case(name: &str) -> String {
    let chars = name.trim_start_matches("r#").chars().collect::<Vec<_>>();
    let mut result = String::new();

    for (index, &ch) in chars.iter().enumerate() {
        if ch.is_uppercase() && index > 0 && chars[index - 1] != '_' {
            let follows_word = chars[index - 1].is_lowercase() || chars[index - 1].is_numeric();
            let ends_acronym = chars.get(index + 1).is_some_and(|next| next.is_lowercase());
            if follows_word || ends_acronym {
                result.push('_');
            }
        }
        result.extend(ch.to_lowercase());
    }

    if matches!(result.as_str(), "self" | "super" | "crate") {
        result.push('_');
    }
    result
}
