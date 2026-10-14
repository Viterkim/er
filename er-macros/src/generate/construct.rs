use crate::fields::with_fields;
use crate::input::{Case, Field, IdentifierNames, Input, attrs::FieldDefault};
use crate::names::{binding, plain, shield_bindings, snake_case};
use proc_macro2::TokenStream;
use quote::{ToTokens, format_ident, quote};
use std::collections::HashSet;
use syn::{Data, Generics, Ident, Type, visit::Visit as _};

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
    let mut bindings = Vec::new();

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

        let arguments = constructor_arguments(input, case);
        let parameters = arguments.iter().map(|argument| &argument.parameter);
        let values: Vec<_> = arguments
            .iter()
            .map(|argument| argument.value.clone())
            .collect();
        bindings.extend(arguments.iter().map(|argument| argument.binding.clone()));
        let values = located_values(case, values);
        let tracking = caller_tracking(case);
        let body = with_fields(case, target, &values);

        methods.push(quote! {
            #[must_use]
            #tracking
            #[allow(clippy::too_many_arguments, unreachable_code)]
            pub fn #method(#(#parameters),*) -> Self {
                #body
            }
        });
    }

    let empty_from = empty_from(input);

    let implementation = quote! {
        impl #impl_generics #name #type_generics #where_clause {
            #(#methods)*
        }
        #empty_from
    };
    if bindings.is_empty() {
        return Ok(implementation);
    }

    let shields = shield_bindings(&bindings);
    Ok(quote! {
        const _: () = {
            #shields

            #implementation
        };
    })
}

pub fn empty_from(input: &Input<'_>) -> TokenStream {
    if input.cases.len() != 1
        || input.cases[0].variant.is_some()
        || input.cases[0]
            .fields
            .iter()
            .any(|field| !field.options.src_location)
    {
        return TokenStream::new();
    }

    let name = &input.item.ident;
    let (impl_generics, type_generics, where_clause) = input.item.generics.split_for_impl();
    let case = &input.cases[0];
    let values = located_values(case, Vec::new());
    let tracking = caller_tracking(case);
    let body = with_fields(case, quote!(Self), &values);
    quote! {
        impl #impl_generics ::core::convert::From<()> for #name #type_generics #where_clause {
            #tracking
            fn from(_: ()) -> Self {
                #body
            }
        }
    }
}

pub struct ConstructorArgument {
    pub parameter: TokenStream,
    pub value: TokenStream,
    pub binding: Ident,
}

pub fn constructor_arguments(input: &Input<'_>, case: &Case<'_>) -> Vec<ConstructorArgument> {
    let mut dependencies = IdentifierNames::default();
    dependencies.visit_generics(&input.item.generics);
    for field in &case.fields {
        dependencies.visit_type(&field.item.ty);
    }
    let reserved = &input.names;

    let mut arguments = Vec::with_capacity(case.fields.len());
    for (index, field) in case.fields.iter().enumerate() {
        if field.options.src_location {
            continue;
        }

        let ty = &field.item.ty;
        let mut argument = match &field.item.ident {
            Some(name) => name.clone(),
            None if case.fields.len() == 1 => format_ident!("value"),
            None => format_ident!("value{index}"),
        };
        let text = plain(&argument);
        if snake_case(&text) != text || dependencies.names.contains(&text) {
            argument = binding(reserved, &format!("__er_arg_{index}"));
        }
        let (parameter, value) = if field.options.exact {
            (quote!(#argument: #ty), quote!(#argument))
        } else {
            (
                quote!(#argument: impl ::core::convert::Into<#ty>),
                quote!(::core::convert::Into::into(#argument)),
            )
        };
        arguments.push(ConstructorArgument {
            parameter,
            value,
            binding: argument,
        });
    }

    arguments
}

pub fn from_fields(input: &Input<'_>) -> TokenStream {
    let name = &input.item.ident;
    let mut generics = input.item.generics.clone();
    let (_, type_generics, _) = input.item.generics.split_for_impl();

    match &input.item.data {
        Data::Enum(_) => {
            let (impl_generics, _, where_clause) = generics.split_for_impl();
            let error = binding(&input.names, "__er_error");
            let shields = shield_bindings([&error]);
            return quote! {
                const _: () = {
                    #shields

                    impl #impl_generics ::core::convert::From<(Self,)> for #name #type_generics #where_clause {
                        fn from((#error,): (Self,)) -> Self {
                            #error
                        }
                    }
                };
            };
        }
        Data::Struct(_) if !input.options.no_constructors => {}
        Data::Struct(_) | Data::Union(_) => return TokenStream::new(),
    }

    let case = &input.cases[0];
    let inputs = field_inputs(
        case.fields
            .iter()
            .enumerate()
            .filter(|(_, field)| !field.options.src_location),
        &mut generics,
        &input.names,
    );
    let payload = tuple_input(&inputs.types);
    let pattern = tuple_input(&inputs.bindings);
    let values = located_values(case, inputs.values);
    let tracking = caller_tracking(case);
    let body = with_fields(case, quote!(Self), &values);
    let (impl_generics, _, where_clause) = generics.split_for_impl();
    let shields = shield_bindings(&inputs.bindings);

    let defaults = from_defaults(input, case);

    // The extra tuple keeps this clear of From<T> for T, even for boxed fields.
    quote! {
        #defaults
        const _: () = {
            #shields

            impl #impl_generics ::core::convert::From<(#payload,)> for #name #type_generics #where_clause {
                #[allow(unreachable_code)]
                #tracking
                fn from((#pattern,): (#payload,)) -> Self {
                    #body
                }
            }
        };
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

pub fn default_value(field: &Field<'_>, generics: &mut Generics) -> Option<TokenStream> {
    if field.options.src_location {
        return Some(quote!(::core::panic::Location::caller()));
    }

    match &field.options.default {
        Some(FieldDefault::Trait) => {
            let ty = &field.item.ty;
            generics
                .make_where_clause()
                .predicates
                .push(syn::parse_quote!(
                    #ty: ::core::default::Default
                ));
            Some(quote!(<#ty as ::core::default::Default>::default()))
        }
        Some(FieldDefault::Expression(expression)) => Some(quote!(#expression)),
        None => None,
    }
}

pub fn from_defaults(input: &Input<'_>, case: &Case<'_>) -> TokenStream {
    if !case
        .fields
        .iter()
        .any(|field| field.options.default.is_some())
    {
        return TokenStream::new();
    }

    let name = &input.item.ident;
    let mut generics = input.item.generics.clone();
    let (_, type_generics, _) = input.item.generics.split_for_impl();
    let required: Vec<_> = case
        .fields
        .iter()
        .enumerate()
        .filter(|(_, field)| field.options.default.is_none() && !field.options.src_location)
        .collect();
    let inputs = field_inputs(required.iter().copied(), &mut generics, &input.names);
    let mut values = vec![TokenStream::new(); case.fields.len()];
    for ((index, _), value) in required.iter().zip(inputs.values) {
        values[*index] = value;
    }
    for (index, field) in case.fields.iter().enumerate() {
        if let Some(value) = default_value(field, &mut generics) {
            values[index] = value;
        }
    }

    let body = with_fields(case, quote!(Self), &values);
    let (impl_generics, _, where_clause) = generics.split_for_impl();
    let tracking = caller_tracking(case);
    let empty = if required.is_empty() {
        quote! {
            impl #impl_generics ::core::convert::From<()> for #name #type_generics #where_clause {
                #tracking
                fn from(_: ()) -> Self {
                    #body
                }
            }
        }
    } else {
        TokenStream::new()
    };

    let shields = shield_bindings(&inputs.bindings);
    let payload = tuple_input(&inputs.types);
    let pattern = tuple_input(&inputs.bindings);

    // A distinct source shape keeps the shortened and full conversions disjoint.
    quote! {
        #empty
        const _: () = {
            #shields

            impl #impl_generics ::core::convert::From<(#payload, [(); 1])> for #name #type_generics #where_clause {
                #tracking
                fn from((#pattern, _): (#payload, [(); 1])) -> Self {
                    #body
                }
            }
        };
    }
}

pub fn caller_tracking(case: &Case<'_>) -> TokenStream {
    if case.fields.iter().any(|field| field.options.src_location) {
        quote!(#[track_caller])
    } else {
        TokenStream::new()
    }
}

pub fn located_values(case: &Case<'_>, values: Vec<TokenStream>) -> Vec<TokenStream> {
    let mut values = values.into_iter();
    case.fields
        .iter()
        .map(|field| {
            if field.options.src_location {
                quote!(::core::panic::Location::caller())
            } else {
                values.next().unwrap_or_default()
            }
        })
        .collect()
}
