use super::{impl_scope, replace_self::ReplaceSelf};
use crate::fields::with_fields;
use crate::input::{Case, Constructors, Field, Input, attrs::FieldDefault, format::FormatBody};
use crate::names::{binding, plain, shield_bindings, snake_case};
use proc_macro2::{Delimiter, Group, TokenStream};
use quote::{ToTokens, format_ident, quote};
use std::collections::HashSet;
use syn::visit_mut::VisitMut as _;
use syn::{GenericParam, Generics, Ident, Path};

pub fn implementations(
    input: &Input<'_>,
    with_fields: bool,
    bindings: &mut Vec<Ident>,
) -> syn::Result<(TokenStream, TokenStream)> {
    if input.options.no_constructors {
        return Ok((TokenStream::new(), TokenStream::new()));
    }

    let mut field_names = Vec::new();
    let mut methods = methods(input, &mut field_names)?;
    let (mut defaults, fields) = if with_fields {
        from_fields(input, &input.options.er_path, &field_names, bindings)
    } else {
        (TokenStream::new(), TokenStream::new())
    };

    if field_names.is_empty() {
        defaults.extend(fields);
        return Ok((defaults, methods));
    }

    methods.extend(fields);
    defaults.extend(scoped(input, &field_names, methods));
    Ok((defaults, TokenStream::new()))
}

pub fn methods(input: &Input<'_>, bindings: &mut Vec<Ident>) -> syn::Result<TokenStream> {
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
        let vis = match case.constructors {
            Constructors::TypeVisibility => &input.item.vis,
            Constructors::Restricted(vis) => vis,
            Constructors::Disabled | Constructors::NonExhaustive => continue,
        };
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
        let wide = (parameters.len() > 7).then(|| quote!(#[allow(clippy::too_many_arguments)]));

        methods.extend(quote! {
            #must_use
            #tracking
            #wide
            #vis fn #method(#(#parameters),*) -> Self {
                #body
            }
        });
    }

    if methods.is_empty() {
        return Ok(TokenStream::new());
    }

    let methods = Group::new(Delimiter::Brace, methods);
    Ok(quote! {
        #[automatically_derived]
        impl #impl_generics #name #type_generics #where_clause #methods
    })
}

fn argument_name(input: &Input<'_>, case: &Case<'_>, index: usize, field: &Field<'_>) -> Ident {
    let name = match &field.item.ident {
        Some(name) => {
            let mut argument = name.clone();
            argument.set_span(proc_macro2::Span::mixed_site());
            argument
        }
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

pub fn from_fields(
    input: &Input<'_>,
    er_path: &Path,
    arguments: &[Ident],
    bindings: &mut Vec<Ident>,
) -> (TokenStream, TokenStream) {
    let Some(case) = input.cases.first().filter(|case| {
        case.variant.is_none() && matches!(case.constructors, Constructors::TypeVisibility)
    }) else {
        return (TokenStream::new(), TokenStream::new());
    };

    let name = &input.item.ident;
    let mut generics = input.item.generics.clone();
    let (_, type_generics, _) = input.item.generics.split_for_impl();

    let mut types = Vec::with_capacity(case.fields.len());
    let mut bounds = Vec::new();
    for (index, field) in case.fields.iter().enumerate() {
        if field.options.src_location {
            continue;
        }

        let (ty, bound) = input_type(index, field, &mut generics, &input.names);
        types.push(ty);
        bounds.extend(bound);
    }

    let payload = tuple_input(&types);
    let tracking = caller_tracking(case);
    let (impl_generics, _, _) = generics.split_for_impl();
    let where_clause = where_with(&generics, &bounds);

    let pattern = tuple_input(arguments);
    let defaults = from_defaults(input, case, er_path, bindings);
    let field_target = field_target(input, er_path, None);

    let fields = quote! {
        #field_target
        #[automatically_derived]
        impl #impl_generics #er_path::er_internal::ErFromFields<#payload> for #name #type_generics #where_clause {
            #tracking
            fn er_from_fields(#pattern: #payload) -> Self {
                Self::new(#(#arguments),*)
            }
        }
    };

    (defaults, fields)
}

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
    // A field named `check` imports `drop as check`. Keep it out of user code like `#[er(default = check())]`.
    let shields = shield_bindings(field_names);
    if has_user_code(input) {
        impl_scope([shields, items])
    } else {
        [shields, items].into_iter().collect()
    }
}

fn field_target(input: &Input<'_>, er_path: &Path, form: Option<&TokenStream>) -> TokenStream {
    let form = form.map(|form| quote!(, #form));
    let name = &input.item.ident;
    let (_, type_generics, _) = input.item.generics.split_for_impl();
    let target = quote!(#name #type_generics);
    let payload = binding(&input.names, "__ErFields");

    let mut generics = input.item.generics.clone();
    if input.names.contains("Self") {
        let original = syn::parse_quote!(#target);
        ReplaceSelf {
            original: &original,
        }
        .visit_generics_mut(&mut generics);
    }
    let impl_generics = if generics.params.is_empty() {
        quote!(<#payload>)
    } else {
        generics
            .params
            .push(GenericParam::Type(payload.clone().into()));
        let (params, _, _) = generics.split_for_impl();
        quote!(#params)
    };
    let predicates = generics
        .where_clause
        .iter()
        .flat_map(|clause| &clause.predicates);

    quote! {
        impl #impl_generics #er_path::er_internal::ErFieldsFor<#target #form>
            for #er_path::er_internal::ErFieldInput<#payload>
        where #(#predicates,)* #target: #er_path::er_internal::ErFromFields<#payload #form> {}
    }
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

fn default_value(default: &FieldDefault, ty: &syn::Type, generics: &mut Generics) -> TokenStream {
    match default {
        FieldDefault::Trait => {
            generics
                .make_where_clause()
                .predicates
                .push(syn::parse_quote!(#ty: ::core::default::Default));
            quote!(<#ty as ::core::default::Default>::default())
        }
        FieldDefault::Expression(expression @ (syn::Expr::Lit(_) | syn::Expr::Path(_))) => {
            quote!(#expression)
        }
        // A default can return the whole error. Give rustc another path through the struct literal
        // so it doesn't flag our generated fields as unreachable on 1.89.
        FieldDefault::Expression(expression) => quote! {
            '__er_default: {
                {
                    use ::core::mem::drop as __er_default_value;
                    if let ::core::option::Option::Some(__er_default_value) = ::core::option::Option::<#ty>::None {
                        break '__er_default __er_default_value;
                    }
                }
                #expression
            }
        },
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

    // The full tuple already has its own impl.
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

    let mut supplied_defaults = given;
    let mut arguments = Vec::new();
    let mut types = Vec::new();
    let mut bounds = Vec::new();
    let mut values = Vec::with_capacity(case.fields.len());

    for (index, field) in case.fields.iter().enumerate() {
        if field.options.src_location {
            values.push(quote!(::core::panic::Location::caller()));
            continue;
        }

        if let Some(default) = &field.options.default {
            if supplied_defaults == 0 {
                values.push(default_value(default, &field.item.ty, &mut generics));
                continue;
            }
            supplied_defaults -= 1;
        }

        let (ty, bound) = input_type(index, field, &mut generics, &input.names);
        let argument = binding(&input.names, &format!("__er_value_{index}"));
        let value = if field.options.exact {
            quote!(#argument)
        } else {
            quote!(::core::convert::Into::into(#argument))
        };

        types.push(ty);
        bounds.extend(bound);
        arguments.push(argument);
        values.push(value);
    }

    let body = with_fields(case, quote!(Self), &values);
    let (impl_generics, _, _) = generics.split_for_impl();
    let where_clause = where_with(&generics, &bounds);
    let tracking = caller_tracking(case);
    let payload = tuple_input(&types);
    let pattern = tuple_input(&arguments);
    bindings.extend(arguments);

    let form = quote!(#er_path::er_internal::ErDefaults<#given>);
    let field_target = field_target(input, er_path, Some(&form));

    quote! {
        #field_target
        #[automatically_derived]
        impl #impl_generics #er_path::er_internal::ErFromFields<#payload, #form> for #name #type_generics #where_clause {
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
