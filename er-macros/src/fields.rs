use crate::input::Case;
use proc_macro2::{TokenStream, TokenTree};
use quote::quote;
use syn::Fields;

// Works for both construction and matching.
pub fn with_fields(case: &Case<'_>, target: TokenStream, values: &[TokenStream]) -> TokenStream {
    match case.shape {
        Fields::Unit => target,
        Fields::Unnamed(_) => quote!(#target(#(#values),*)),
        Fields::Named(_) => {
            let fields = case.fields.iter().zip(values).map(|(field, value)| {
                let name = &field.item.ident;
                let mut tokens = value.clone().into_iter();

                // Keep the derive from triggering redundant_field_names on the user's fields.
                if let Some(TokenTree::Ident(value_name)) = tokens.next()
                    && tokens.next().is_none()
                    && name.as_ref() == Some(&value_name)
                {
                    quote!(#name)
                } else {
                    quote!(#name: #value)
                }
            });
            quote!(#target { #(#fields),* })
        }
    }
}
