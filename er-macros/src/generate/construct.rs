use super::replace_self::ReplaceSelf;
use crate::fields::with_fields;
use crate::input::{Case, Field, Input, attrs::FieldDefault, format::FormatBody};
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
                    format!(
                        "two things want the constructor `{text}`, rename one so they stop fighting"
                    ),
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

    let field_target = field_target(input, er_path, quote!(#er_path::er_internal::ErFields));

    let fields = quote! {
        #field_target
        impl #impl_generics #er_path::er_internal::ErFromFields<#payload, #er_path::er_internal::ErFields> for #name #type_generics #where_clause {
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
/// call `drop`. So with a default expression or a format function around, the field names only get shielded in
/// a scope of their own. Without one, the shields can sit next to everything else and save a `const` item.
fn has_user_code(input: &Input<'_>) -> bool {
    input.cases.iter().any(|case| {
        let formats = &case.formats;
        let function = [
            &formats.shared,
            &formats.display,
            &formats.display_alt,
            &formats.debug,
        ]
        .into_iter()
        .flatten()
        .any(|format| match format {
            FormatBody::Function(_) => true,
            FormatBody::Template(_) => false,
        });
        let expression = case
            .fields
            .iter()
            .filter_map(|field| field.options.default.as_ref())
            .any(|default| match default {
                FieldDefault::Expression(_) => true,
                FieldDefault::Trait => false,
            });

        function || expression
    })
}

pub fn scoped(input: &Input<'_>, field_names: &[Ident], items: TokenStream) -> TokenStream {
    if field_names.is_empty() || items.is_empty() {
        return items;
    }

    let shields = shield_bindings(field_names);
    if !has_user_code(input) {
        return quote!(#shields #items);
    }

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
    let accepts = quote!(#target: #er_path::er_internal::ErFromFields<#payload, #form>);
    let where_clause = where_with(&generics, &[accepts]);

    quote! {
        impl #impl_generics #er_path::er_internal::ErFieldsFor<#target, #form>
            for #er_path::er_internal::ErFieldInput<#payload> #where_clause {}
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
    let defaults = case
        .fields
        .iter()
        .filter(|field| field.options.default.is_some())
        .count();

    // One impl per number of defaults given, they're the last fields so each one leaves out a few more from
    // the end. Giving all of them is the `ErFields` impl.
    (0..defaults)
        .map(|given| from_some_defaults(input, case, er_path, bindings, given))
        .collect()
}

fn from_some_defaults(
    input: &Input<'_>,
    case: &Case<'_>,
    er_path: &Path,
    bindings: &mut Vec<Ident>,
    given: usize,
) -> TokenStream {
    let name = &input.item.ident;
    let mut generics = input.item.generics.clone();
    let (_, type_generics, _) = input.item.generics.split_for_impl();

    let mut defaults_left = given;
    let arguments: Vec<_> = case
        .fields
        .iter()
        .enumerate()
        .filter(|(_, field)| !field.options.src_location)
        .filter(|(_, field)| match field.options.default {
            Some(_) if defaults_left == 0 => false,
            Some(_) => {
                defaults_left -= 1;
                true
            }
            None => true,
        })
        .collect();
    let inputs = field_inputs(arguments.iter().copied(), &mut generics, &input.names);

    let mut values = vec![TokenStream::new(); case.fields.len()];
    for (index, field) in case.fields.iter().enumerate() {
        if let Some(value) = default_value(field, &mut generics) {
            values[index] = value;
        }
    }
    for ((index, _), value) in arguments.iter().zip(inputs.values) {
        values[*index] = value;
    }

    let body = with_fields(case, quote!(Self), &values);
    let (impl_generics, _, _) = generics.split_for_impl();
    let where_clause = where_with(&generics, &inputs.bounds);
    let tracking = caller_tracking(case);
    let payload = tuple_input(&inputs.types);
    let pattern = tuple_input(&inputs.bindings);
    bindings.extend(inputs.bindings);

    let form = quote!(#er_path::er_internal::ErDefaults<#given>);
    let field_target = field_target(input, er_path, form.clone());

    quote! {
        #field_target
        impl #impl_generics #er_path::er_internal::ErFromFields<#payload, #form> for #name #type_generics #where_clause {
            // A default like `todo!()` would flag every field after it.
            #[allow(unreachable_code)]
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
