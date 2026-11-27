pub mod construct;
pub mod replace_self;
pub mod source;
pub mod warnings;
pub mod wrap;

use crate::{
    format,
    input::{Input, bounds::format_generics},
    names::{binding, shield_bindings},
};
use proc_macro2::{Delimiter, Group, TokenStream};
use quote::quote;
use syn::DeriveInput;

pub fn expand(item: &DeriveInput) -> syn::Result<TokenStream> {
    let input = Input::parse(item)?;
    let er_path = &input.options.er_path;
    let generics = format_generics(&input);
    let formatter = binding(&input.names, "__er_f");
    let mut bindings = Vec::new();
    let formatting = format::implementations(&input, &generics, &formatter, &mut bindings);
    let error = source::implementation(&input, &generics, er_path, &mut bindings);
    let (constructor_impls, constructors) =
        construct::implementations(&input, true, &mut bindings)?;
    let shields = shield_bindings(&bindings);
    let warnings = warnings::warnings(&input);

    let wrap = match &input.options.wrap {
        Some(options) => wrap::expand(item, er_path, options, &input.names, &formatter),
        None => TokenStream::new(),
    };

    let mut output = impl_scope([shields, formatting, error, constructor_impls, warnings]);
    output.extend(constructors);
    output.extend(wrap);
    Ok(output)
}

pub fn impl_scope(items: impl IntoIterator<Item = TokenStream>) -> TokenStream {
    let body = Group::new(Delimiter::Brace, items.into_iter().collect());
    quote!(const _: () = #body;)
}
