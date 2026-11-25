use crate::fields::with_fields;
use crate::input::{
    Case, Input,
    attrs::FieldMode,
    format::{Format, FormatBody, Output},
};
use crate::names::{binding, plain};
use proc_macro2::{Span, TokenStream};
use quote::{quote, quote_spanned};
use std::mem::discriminant;
use syn::{Data, Fields, Ident, Index, LitStr, Member, spanned::Spanned as _};

pub fn display(input: &Input<'_>, formatter: &Ident) -> TokenStream {
    let debug = quote!(::core::fmt::Debug::fmt(self, #formatter));
    let mut arms = Vec::new();
    let mut fallback = false;

    for case in &input.cases {
        let formats = &case.formats;
        let separate = formats.display.is_some() || formats.debug.is_some();
        let alternate = formats
            .display_alt
            .as_ref()
            .map(|_| case_body(input, case, Output::DisplayAlt, formatter));

        if case.variant.is_none() {
            let normal = if separate {
                case_body(input, case, Output::Display, formatter)
            } else {
                debug
            };
            return match alternate {
                Some(body) => quote!(if #formatter.alternate() { #body } else { #normal }),
                None => normal,
            };
        }

        if separate || alternate.is_some() {
            let pattern = variant_pattern(input, case);
            if let Some(body) = alternate {
                arms.push(quote!(#pattern if #formatter.alternate() => { #body }));
            }
            if separate {
                let body = case_body(input, case, Output::Display, formatter);
                arms.push(quote!(#pattern => { #body }));
            }
        }
        fallback |= !separate;
    }

    if arms.is_empty() {
        return debug;
    }
    if fallback {
        arms.push(quote!(_ => #debug));
    }
    quote!(match self { #(#arms),* })
}

pub fn debug(input: &Input<'_>, formatter: &Ident) -> TokenStream {
    if let Data::Struct(_) = &input.item.data {
        return case_body(input, &input.cases[0], Output::Debug, formatter);
    }

    if input.cases.is_empty() {
        return quote!(match *self {});
    }

    let labels: Option<Vec<_>> = input
        .cases
        .iter()
        .map(
            |case| match case.formats.debug.as_ref().or(case.formats.shared.as_ref()) {
                None if !case.fields.iter().any(|field| field.printed_by_default()) => {
                    Some(case_label(input, case))
                }
                Some(FormatBody::Template(format)) if format.arguments.is_empty() => {
                    Some(literal_text(format))
                }
                _ => None,
            },
        )
        .collect();
    if let Some(labels) = labels {
        let variants = input.cases.iter().map(|case| case.variant);
        let rest = quote!({ .. });
        let fields = input
            .cases
            .iter()
            .map(|case| (!matches!(case.shape, Fields::Unit)).then_some(&rest));
        return quote!(#formatter.write_str(match self { #(Self::#variants #fields => #labels),* }));
    }

    let shape = input.cases[0].shape;
    let helper = match shape {
        Fields::Named(_) => Some(quote!(debug_field)),
        Fields::Unnamed(_) => Some(quote!(debug_tuple_field)),
        Fields::Unit => None,
    };
    if let Some(helper) = helper
        && input.cases.iter().all(|case| {
            discriminant(case.shape) == discriminant(shape)
                && case.formats.debug.is_none()
                && case.formats.shared.is_none()
                && case
                    .fields
                    .iter()
                    .filter(|field| field.printed_by_default())
                    .count()
                    == 1
        })
    {
        let er_path = &input.options.er_path;
        let arms = input.cases.iter().map(|case| {
            let pattern = variant_pattern(input, case);
            let label = case_label(input, case);
            let fields = default_fields(input, case);
            let field = &fields[0];
            quote!(#pattern => (#label, #field))
        });

        return quote!(#er_path::er_internal::impls::format::#helper(#formatter, match self { #(#arms),* }));
    }

    let arms = input.cases.iter().map(|case| {
        let pattern = variant_pattern(input, case);
        let body = case_body(input, case, Output::Debug, formatter);
        quote!(#pattern => { #body })
    });

    quote!(match self { #(#arms),* })
}

pub fn case_body(
    input: &Input<'_>,
    case: &Case<'_>,
    output: Output,
    formatter: &Ident,
) -> TokenStream {
    let tried = case.formats.tried(output);
    let mut body = if tried.default {
        default_body(input, case, formatter)
    } else {
        TokenStream::new()
    };

    for format in tried.formats.into_iter().rev() {
        body = match format {
            FormatBody::Template(format) => template_body(input, case, format, formatter, body),
            FormatBody::Function(path) => quote!(#path(self, #formatter)),
        };
    }

    body
}

fn variant_pattern(input: &Input<'_>, case: &Case<'_>) -> TokenStream {
    let mut patterns = Vec::new();
    for (index, field) in case.fields.iter().enumerate() {
        let used = !field.formats.is_empty() || field.used_in_format;
        let pattern = if used && field.options.mode == FieldMode::Normal {
            let local = binding(&input.names, &format!("__er_{index}"));
            quote!(#local)
        } else {
            quote!(_)
        };
        patterns.push(pattern);
    }

    let variant = case.variant;
    with_fields(case, quote!(Self::#variant), &patterns)
}

pub fn template_body(
    input: &Input<'_>,
    case: &Case<'_>,
    format: &Format,
    formatter: &Ident,
    otherwise: TokenStream,
) -> TokenStream {
    if format.arguments.is_empty() {
        let literal = literal_text(format);

        return quote!(#formatter.write_str(#literal));
    }

    let mut optional = Vec::new();
    let mut arguments = Vec::new();

    for argument in &format.arguments {
        let field = &case.fields[argument.field];
        let value = if argument.optional {
            if !optional.contains(&argument.field) {
                optional.push(argument.field);
            }

            let inside = binding(&input.names, &format!("__er_some_{}", argument.field));
            quote!(*#inside)
        } else {
            field_value(input, case, argument.field)
        };

        if argument.members.is_empty() || field.options.mode == FieldMode::Censor {
            arguments.push(value);
        } else {
            let members = &argument.members;
            arguments.push(quote_spanned!(field.item.ty.span()=> (#value)#(.#members)*));
        }
    }

    let literal = &format.literal;
    let write = quote!(#formatter.write_fmt(::core::format_args!(#literal, #(#arguments),*)));

    if optional.is_empty() {
        return write;
    }

    let values = optional
        .iter()
        .map(|index| field_value(input, case, *index));
    let insides = optional
        .iter()
        .map(|index| binding(&input.names, &format!("__er_some_{index}")));

    quote! {
        match (#(&#values,)*) {
            (#(::core::option::Option::Some(#insides),)*) => #write,
            _ => #otherwise,
        }
    }
}

fn literal_text(format: &Format) -> LitStr {
    let text = format.literal.value().replace("{{", "{").replace("}}", "}");
    LitStr::new(&text, format.literal.span())
}

fn default_fields(input: &Input<'_>, case: &Case<'_>) -> Vec<TokenStream> {
    let mut fields = Vec::new();
    for (index, field) in case.fields.iter().enumerate() {
        if !field.printed_by_default() {
            continue;
        }

        let value = field_value(input, case, index);

        let reference = if case.variant.is_none() && index + 1 == case.fields.len() {
            quote_spanned!(field.item.ty.span()=> &&#value)
        } else {
            quote_spanned!(field.item.ty.span()=> &#value)
        };
        if let Some(name) = &field.item.ident {
            let label = LitStr::new(&plain(name), name.span());
            fields.push(quote!((#label, #reference)));
        } else {
            fields.push(reference);
        }
    }

    fields
}

pub fn default_body(input: &Input<'_>, case: &Case<'_>, formatter: &Ident) -> TokenStream {
    let fields = default_fields(input, case);
    let label = case_label(input, case);
    if fields.is_empty() {
        return quote!(#formatter.write_str(#label));
    }

    let er_path = &input.options.er_path;
    if matches!(case.shape, Fields::Named(_)) {
        if let [field] = fields.as_slice() {
            return quote!(#er_path::er_internal::impls::format::debug_field(#formatter, (#label, #field)));
        }

        quote!(#er_path::er_internal::impls::format::debug_struct(#formatter, #label, &[#(#fields),*]))
    } else {
        if let [field] = fields.as_slice() {
            return quote!(#er_path::er_internal::impls::format::debug_tuple_field(#formatter, (#label, #field)));
        }

        quote!(#er_path::er_internal::impls::format::debug_tuple(#formatter, #label, &[#(#fields),*]))
    }
}

fn case_label(input: &Input<'_>, case: &Case<'_>) -> LitStr {
    let mut label = plain(&input.item.ident);
    if let Some(variant) = case.variant {
        label.push_str("::");
        label.push_str(&plain(variant));
    }

    let name = case.variant.unwrap_or(&input.item.ident);
    LitStr::new(&label, name.span())
}

pub fn field_value(input: &Input<'_>, case: &Case<'_>, index: usize) -> TokenStream {
    let field = &case.fields[index];
    if field.options.mode == FieldMode::Censor {
        let er_path = &input.options.er_path;
        return quote!(#er_path::er_internal::impls::format::Censored);
    }

    if case.variant.is_some() {
        let local = binding(&input.names, &format!("__er_{index}"));
        quote_spanned!(field.item.ty.span()=> *#local)
    } else {
        let member = match &field.item.ident {
            Some(name) => Member::Named(name.clone()),
            None => Member::Unnamed(Index::from(index)),
        };
        // `self` resolves like our code and only points at the field. With the field type's whole span it belongs
        // to whichever macro wrote the struct, and stops naming the `&self` we put on the method.
        let span = field.item.ty.span();
        let this = Ident::new("self", Span::call_site().located_at(span));
        quote_spanned!(span=> #this.#member)
    }
}
