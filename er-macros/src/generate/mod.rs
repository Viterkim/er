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
use proc_macro2::TokenStream;
use quote::quote;
use syn::DeriveInput;

pub fn expand(item: &DeriveInput) -> syn::Result<TokenStream> {
    let input = Input::parse(item)?;
    let er_path = &input.options.er_path;
    let generics = format_generics(&input);
    let formatter = binding(&input.names, "__er_f");
    let mut bindings = Vec::new();
    let mut field_names = Vec::new();
    let formatting = format::implementations(&input, &generics, &formatter, &mut bindings);
    let (shielded, constructors) = construct::methods_in_scope(&input, &mut field_names)?;
    let error = source::implementation(&input, &generics, er_path, &mut bindings);
    let (defaults, fields) = construct::from_fields(&input, er_path, &mut bindings);
    let shields = shield_bindings(&bindings);
    let by_field_name = construct::scoped(&input, &field_names, quote!(#shielded #fields));
    let warnings = warnings::warnings(&input);

    let wrap = match &input.options.wrap {
        Some(options) => wrap::expand(item, er_path, options, &input.names, &formatter),
        None => TokenStream::new(),
    };

    Ok(quote! {
        const _: () = {
            #shields

            #formatting
            #error
            #defaults
            #by_field_name
            #warnings
        };
        #constructors
        #wrap
    })
}
