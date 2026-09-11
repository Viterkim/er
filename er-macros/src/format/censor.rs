use crate::input::{Input, attrs::FieldMode};
use proc_macro2::TokenStream;
use quote::quote;
use syn::Ident;

pub fn helper(input: &Input<'_>, formatter: &Ident) -> TokenStream {
    let mut needed = Vec::new();
    for case in &input.cases {
        for field in &case.fields {
            if field.options.mode != FieldMode::Censor {
                continue;
            }
            for format in &field.formats {
                if !needed.contains(format) {
                    needed.push(*format);
                }
            }
        }
    }

    if needed.is_empty() {
        return TokenStream::new();
    }

    let traits = needed.into_iter().map(|format| format.ident());

    quote! {
        struct __ErCensored;
        #(impl ::core::fmt::#traits for __ErCensored {
            fn fmt(&self, #formatter: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                #formatter.write_str("*CENSORED*")
            }
        })*
    }
}
