use crate::input::{Input, bounds::format_generics};
use quote::ToTokens as _;
use syn::Type;

#[test]
pub fn token_types_keep_bounds() -> syn::Result<()> {
    let mut item: syn::DeriveInput = syn::parse_quote! {
        pub struct Payload<T> { pub value: selected!(T) }
    };
    let input = Input::parse(&item)?;
    let generics = format_generics(&input)?;
    let expected: syn::WherePredicate = syn::parse_quote!(selected!(T): ::core::fmt::Debug);
    let where_clause = generics
        .where_clause
        .ok_or_else(|| syn::Error::new_spanned(&item, "missing where clause"))?;
    assert_eq!(
        where_clause.predicates.to_token_stream().to_string(),
        expected.to_token_stream().to_string()
    );

    let syn::Data::Struct(data) = &mut item.data else {
        return Err(syn::Error::new_spanned(&item, "expected a struct"));
    };
    let field = data
        .fields
        .iter_mut()
        .next()
        .ok_or_else(|| syn::Error::new(proc_macro2::Span::call_site(), "missing field"))?;
    field.ty = Type::Verbatim(quote::quote!(T));

    let input = Input::parse(&item)?;
    let generics = format_generics(&input)?;
    let expected: syn::WherePredicate = syn::parse_quote!(T: ::core::fmt::Debug);
    let where_clause = generics
        .where_clause
        .ok_or_else(|| syn::Error::new_spanned(&item, "missing where clause"))?;
    assert_eq!(
        where_clause.predicates.to_token_stream().to_string(),
        expected.to_token_stream().to_string()
    );

    Ok(())
}
