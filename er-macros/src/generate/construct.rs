use crate::fields::with_fields;
use crate::input::{Case, Field, Input};
use crate::names::{binding, identifiers, plain, snake_case};
use proc_macro2::TokenStream;
use quote::{ToTokens, format_ident, quote};
use std::collections::HashSet;
use syn::{Data, Generics, Ident, Type};

pub fn methods(input: &Input<'_>) -> syn::Result<TokenStream> {
    if input.options.no_constructors {
        return Ok(TokenStream::new());
    }

    let name = &input.item.ident;
    let (impl_generics, type_generics, where_clause) = input.item.generics.split_for_impl();
    let variants: HashSet<_> = input
        .cases
        .iter()
        .filter_map(|case| case.variant.map(plain))
        .collect();
    let mut methods = Vec::with_capacity(input.cases.len());
    let mut method_names = HashSet::new();

    for case in &input.cases {
        let (method, target) = if let Some(variant) = case.variant {
            let text = snake_case(&plain(variant));
            if text == "er_wrap" && input.options.wrap.is_some() {
                return Err(syn::Error::new_spanned(
                    variant,
                    "constructor `er_wrap` clashes with the generated Wrap method",
                ));
            }

            if !method_names.insert(text.clone()) || variants.contains(&text) {
                return Err(syn::Error::new_spanned(
                    variant,
                    format!("constructor `{text}` clashes with another variant or constructor"),
                ));
            }

            let method = Ident::new_raw(&text, variant.span());
            (method, quote!(Self::#variant))
        } else {
            (format_ident!("new"), quote!(Self))
        };

        let (parameters, values) = constructor_arguments(input, case);

        let body = with_fields(case, target, &values);

        methods.push(quote! {
            #[must_use]
            pub fn #method(#(#parameters),*) -> Self {
                #body
            }
        });
    }

    let empty_from = if input.cases.len() == 1
        && input.cases[0].variant.is_none()
        && input.cases[0].fields.is_empty()
    {
        let body = with_fields(&input.cases[0], quote!(Self), &[]);
        quote! {
            impl #impl_generics ::core::convert::From<()> for #name #type_generics #where_clause {
                fn from(_: ()) -> Self {
                    #body
                }
            }
        }
    } else {
        TokenStream::new()
    };

    Ok(quote! {
        impl #impl_generics #name #type_generics #where_clause {
            #(#methods)*
        }
        #empty_from
    })
}

pub fn constructor_arguments(
    input: &Input<'_>,
    case: &Case<'_>,
) -> (Vec<TokenStream>, Vec<TokenStream>) {
    let mut reserved = input.const_names.clone();
    for field in &case.fields {
        if let Some(name) = &field.item.ident {
            reserved.insert(plain(name));
        }
    }

    let mut parameters = Vec::with_capacity(case.fields.len());
    let mut values = Vec::with_capacity(case.fields.len());
    for (index, field) in case.fields.iter().enumerate() {
        let ty = &field.item.ty;
        let mut argument = match &field.item.ident {
            Some(name) => name.clone(),
            None if case.fields.len() == 1 => format_ident!("value"),
            None => format_ident!("value{index}"),
        };
        let text = plain(&argument);
        if snake_case(&text) != text || input.const_names.contains(&text) {
            argument = binding(&reserved, &format!("__er_arg_{index}"));
        }
        reserved.insert(plain(&argument));

        if field.options.exact {
            parameters.push(quote!(#argument: #ty));
            values.push(quote!(#argument));
        } else {
            parameters.push(quote!(#argument: impl ::core::convert::Into<#ty>));
            values.push(quote!(::core::convert::Into::into(#argument)));
        }
    }

    (parameters, values)
}

pub fn from_fields(input: &Input<'_>) -> TokenStream {
    let name = &input.item.ident;
    let mut generics = input.item.generics.clone();
    let (_, type_generics, _) = input.item.generics.split_for_impl();

    match &input.item.data {
        Data::Enum(_) => {
            let (impl_generics, _, where_clause) = generics.split_for_impl();
            return quote! {
                impl #impl_generics ::core::convert::From<(Self,)> for #name #type_generics #where_clause {
                    fn from((error,): (Self,)) -> Self {
                        error
                    }
                }
            };
        }
        Data::Struct(_) if !input.options.no_constructors => {}
        Data::Struct(_) | Data::Union(_) => return TokenStream::new(),
    }

    let case = &input.cases[0];
    let item = input.item;
    let reserved = identifiers(quote!(#item));
    let inputs = field_inputs(case.fields.iter().enumerate(), &mut generics, &reserved);
    let payload = tuple_input(&inputs.types);
    let pattern = tuple_input(&inputs.bindings);
    let body = with_fields(case, quote!(Self), &inputs.values);
    let (impl_generics, _, where_clause) = generics.split_for_impl();

    // The extra tuple keeps this clear of From<T> for T, even for boxed fields.
    quote! {
        impl #impl_generics ::core::convert::From<(#payload,)> for #name #type_generics #where_clause {
            fn from((#pattern,): (#payload,)) -> Self {
                #body
            }
        }
    }
}

pub struct FieldInputs {
    pub types: Vec<Type>,
    pub bindings: Vec<Ident>,
    pub values: Vec<TokenStream>,
}

pub fn field_inputs<'a, 'item: 'a>(
    fields: impl Iterator<Item = (usize, &'a Field<'item>)>,
    generics: &mut Generics,
    reserved: &HashSet<String>,
) -> FieldInputs {
    let mut inputs = FieldInputs {
        types: Vec::new(),
        bindings: Vec::new(),
        values: Vec::new(),
    };

    for (index, field) in fields {
        let ty = &field.item.ty;
        let value = binding(reserved, &format!("__er_value_{index}"));

        let expression = if field.options.exact {
            inputs.types.push(ty.clone());
            quote!(#value)
        } else {
            let parameter = binding(reserved, &format!("__ErInput{index}"));
            generics.params.push(syn::parse_quote!(#parameter));
            generics
                .make_where_clause()
                .predicates
                .push(syn::parse_quote!(
                    #parameter: ::core::convert::Into<#ty>
                ));
            inputs.types.push(syn::parse_quote!(#parameter));
            quote!(::core::convert::Into::into(#value))
        };

        inputs.bindings.push(value);
        inputs.values.push(expression);
    }

    inputs
}

pub fn tuple_input<T: ToTokens>(items: &[T]) -> TokenStream {
    match items {
        [] => quote!(()),
        [item] => quote!(#item),
        _ => quote!((#(#items),*)),
    }
}
