use super::replace_self::ReplaceSelf;
use crate::fields::with_fields;
use crate::input::{Case, Field, Input, attrs::FieldDefault};
use crate::names::{binding, plain, shield_bindings, snake_case};
use proc_macro2::TokenStream;
use quote::{ToTokens, format_ident, quote};
use std::collections::HashSet;
use syn::fold::Fold as _;
use syn::{Data, GenericParam, Generics, Ident, Path};

pub fn methods_in_scope(
    input: &Input<'_>,
    bindings: &mut Vec<Ident>,
) -> syn::Result<(TokenStream, TokenStream)> {
    let before = bindings.len();
    let methods = methods(input, bindings)?;

    if bindings.len() == before {
        Ok((TokenStream::new(), methods))
    } else {
        Ok((methods, TokenStream::new()))
    }
}

pub fn methods(input: &Input<'_>, bindings: &mut Vec<Ident>) -> syn::Result<TokenStream> {
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
    let mut methods = TokenStream::new();
    let mut method_names = HashSet::with_capacity(variants.len());
    let must_use = quote!(#[must_use]);
    let into = quote!(::core::convert::Into);

    for case in &input.cases {
        let (method, target) = if let Some(variant) = case.variant {
            let text = snake_case(&plain(variant));
            if !method_names.insert(text.clone()) || variants.contains(&text) {
                return Err(syn::Error::new_spanned(
                    variant,
                    format!("two things would get the constructor `{text}`, rename one of them"),
                ));
            }

            let method = Ident::new_raw(&text, variant.span());
            (method, quote!(Self::#variant))
        } else {
            (format_ident!("new"), quote!(Self))
        };

        let mut parameters = Vec::with_capacity(case.fields.len());
        let mut values = Vec::with_capacity(case.fields.len());
        for (index, field) in case.fields.iter().enumerate() {
            if field.options.src_location {
                values.push(quote!(::core::panic::Location::caller()));
                continue;
            }

            let argument = argument_name(input, case, index, field);
            let ty = &field.item.ty;
            if field.options.exact {
                parameters.push(quote!(#argument: #ty));
                values.push(quote!(#argument));
            } else {
                parameters.push(quote!(#argument: impl #into<#ty>));
                values.push(quote!(#into::into(#argument)));
            }
            bindings.push(argument);
        }
        let tracking = caller_tracking(case);
        let body = with_fields(case, target, &values);

        methods.extend(quote! {
            #must_use
            #tracking
            pub fn #method(#(#parameters),*) -> Self {
                #body
            }
        });
    }

    Ok(quote! {
        #[allow(clippy::too_many_arguments, unreachable_code)]
        impl #impl_generics #name #type_generics #where_clause {
            #methods
        }
    })
}

fn argument_name(input: &Input<'_>, case: &Case<'_>, index: usize, field: &Field<'_>) -> Ident {
    let name = match &field.item.ident {
        Some(name) => name.clone(),
        None if case.fields.len() == 1 => format_ident!("value"),
        None => format_ident!("value{index}"),
    };
    let text = plain(&name);
    if snake_case(&text) != text || input.scope_names.contains(&text) {
        binding(&input.names, &format!("__er_arg_{index}"))
    } else {
        name
    }
}

/// The `.er(fields)` helpers, the defaults one first and the full tuple one second.
/// They come apart because the defaults one holds the user's default expressions, see `scoped`.
pub fn from_fields(
    input: &Input<'_>,
    er_path: &Path,
    bindings: &mut Vec<Ident>,
) -> (TokenStream, TokenStream) {
    match &input.item.data {
        Data::Struct(_) if !input.options.no_constructors => {}
        Data::Enum(_) | Data::Struct(_) | Data::Union(_) => {
            return (TokenStream::new(), TokenStream::new());
        }
    }

    let name = &input.item.ident;
    let mut generics = input.item.generics.clone();
    let (_, type_generics, _) = input.item.generics.split_for_impl();
    let case = &input.cases[0];

    let mut types = Vec::with_capacity(case.fields.len());
    let mut arguments = Vec::with_capacity(case.fields.len());
    let mut bounds = Vec::new();
    for (index, field) in case.fields.iter().enumerate() {
        if field.options.src_location {
            continue;
        }

        let (ty, bound) = input_type(index, field, &mut generics, &input.names);
        types.push(ty);
        bounds.extend(bound);
        arguments.push(argument_name(input, case, index, field));
    }
    let payload = tuple_input(&types);
    let tracking = caller_tracking(case);
    let (impl_generics, _, _) = generics.split_for_impl();
    let where_clause = where_with(&generics, &bounds);

    let pattern = tuple_input(&arguments);

    let defaults = from_defaults(input, case, er_path, bindings);

    let field_target = field_target(input, er_path, quote!(#er_path::internal::ErFields));

    let fields = quote! {
        #field_target
        impl #impl_generics #er_path::internal::ErFromFields<#payload, #er_path::internal::ErFields> for #name #type_generics #where_clause {
            #[allow(unreachable_code)]
            #tracking
            fn er_from_fields(#pattern: #payload) -> Self {
                Self::new(#(#arguments),*)
            }
        }
    };

    (defaults, fields)
}

/// Constructor arguments are named after the fields, and a constant with that name would turn the argument
/// into a pattern, so those names get shielded. The shield is an import of `drop` under each name though, and
/// a macro inside a user's default expression that calls `check(..)` next to a field named `check` would quietly
/// call `drop`. So the field names only get shielded around the code that takes them as arguments, where no
/// user expression lives.
pub fn scoped(field_names: &[Ident], items: TokenStream) -> TokenStream {
    if field_names.is_empty() || items.is_empty() {
        return items;
    }

    let shields = shield_bindings(field_names);

    quote! {
        const _: () = {
            #shields

            #items
        };
    }
}

fn field_target(input: &Input<'_>, er_path: &Path, form: TokenStream) -> TokenStream {
    let name = &input.item.ident;
    let (_, type_generics, _) = input.item.generics.split_for_impl();
    let target = quote!(#name #type_generics);
    let payload = binding(&input.names, "__ErFields");

    let mut generics = input.item.generics.clone();
    if input.names.contains("Self") {
        let original = syn::parse_quote!(#target);
        generics = ReplaceSelf {
            original: &original,
        }
        .fold_generics(generics);
    }
    generics
        .params
        .push(GenericParam::Type(payload.clone().into()));
    let (impl_generics, _, _) = generics.split_for_impl();
    let accepts = quote!(#target: #er_path::internal::ErFromFields<#payload, #form>);
    let where_clause = where_with(&generics, &[accepts]);

    quote! {
        impl #impl_generics #er_path::internal::ErFieldsFor<#target, #form>
            for #er_path::internal::ErFieldInput<#payload> #where_clause {}
    }
}

pub struct FieldInputs {
    pub types: Vec<TokenStream>,
    pub bounds: Vec<TokenStream>,
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
        bounds: Vec::new(),
        bindings: Vec::new(),
        values: Vec::new(),
    };

    for (index, field) in fields {
        let (ty, bound) = input_type(index, field, generics, reserved);
        let value = binding(reserved, &format!("__er_value_{index}"));
        let expression = if field.options.exact {
            quote!(#value)
        } else {
            quote!(::core::convert::Into::into(#value))
        };

        inputs.types.push(ty);
        inputs.bounds.extend(bound);
        inputs.bindings.push(value);
        inputs.values.push(expression);
    }

    inputs
}

pub fn input_type(
    index: usize,
    field: &Field<'_>,
    generics: &mut Generics,
    reserved: &HashSet<String>,
) -> (TokenStream, Option<TokenStream>) {
    let ty = &field.item.ty;
    if field.options.exact {
        return (quote!(#ty), None);
    }

    let parameter = binding(reserved, &format!("__ErInput{index}"));
    generics
        .params
        .push(GenericParam::Type(parameter.clone().into()));
    let bound = quote!(#parameter: ::core::convert::Into<#ty>);

    (quote!(#parameter), Some(bound))
}

pub fn where_with(generics: &Generics, bounds: &[TokenStream]) -> TokenStream {
    if generics.where_clause.is_none() && bounds.is_empty() {
        return TokenStream::new();
    }

    let predicates = generics
        .where_clause
        .iter()
        .flat_map(|clause| &clause.predicates);
    quote!(where #(#predicates,)* #(#bounds,)*)
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

pub fn from_defaults(
    input: &Input<'_>,
    case: &Case<'_>,
    er_path: &Path,
    bindings: &mut Vec<Ident>,
) -> TokenStream {
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
    let (impl_generics, _, _) = generics.split_for_impl();
    let where_clause = where_with(&generics, &inputs.bounds);
    let tracking = caller_tracking(case);
    let payload = tuple_input(&inputs.types);
    let pattern = tuple_input(&inputs.bindings);
    bindings.extend(inputs.bindings);

    let field_target = field_target(input, er_path, quote!(#er_path::internal::ErDefaults));

    quote! {
        #field_target
        impl #impl_generics #er_path::internal::ErFromFields<#payload, #er_path::internal::ErDefaults> for #name #type_generics #where_clause {
            #tracking
            fn er_from_fields(#pattern: #payload) -> Self {
                #body
            }
        }
    }
}

pub fn caller_tracking(case: &Case<'_>) -> TokenStream {
    if case.fields.iter().any(|field| field.options.src_location) {
        quote!(#[track_caller])
    } else {
        TokenStream::new()
    }
}
