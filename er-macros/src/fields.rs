use crate::input::Case;
use proc_macro2::TokenStream;
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
                if quote!(#name).to_string() == value.to_string() {
                    quote!(#name)
                } else {
                    quote!(#name: #value)
                }
            });
            quote!(#target { #(#fields),* })
        }
    }
}
