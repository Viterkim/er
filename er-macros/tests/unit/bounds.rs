use crate::input::{Input, bounds::format_generics};
use quote::ToTokens as _;
use syn::Type;

#[test]
pub fn token_bounds() -> syn::Result<()> {
    let mut item: syn::DeriveInput = syn::parse_quote! {
        pub struct Payload<T> { pub value: selected!(T) }
    };
    let input = Input::parse(&item)?;
    let generics = format_generics(&input)?;
    let expected: syn::WherePredicate = syn::parse_quote!(selected!(T): ::core::fmt::Debug);
    assert_eq!(
        generics
            .where_clause
            .map(|clause| clause.predicates.to_token_stream().to_string()),
        Some(expected.to_token_stream().to_string())
    );

    let syn::Data::Struct(data) = &mut item.data else {
        unreachable!()
    };
    for field in data.fields.iter_mut() {
        field.ty = Type::Verbatim(quote::quote!(T));
    }

    let input = Input::parse(&item)?;
    let generics = format_generics(&input)?;
    let expected: syn::WherePredicate = syn::parse_quote!(T: ::core::fmt::Debug);
    assert_eq!(
        generics
            .where_clause
            .map(|clause| clause.predicates.to_token_stream().to_string()),
        Some(expected.to_token_stream().to_string())
    );

    Ok(())
}
