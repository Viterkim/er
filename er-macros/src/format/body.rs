use crate::fields::with_fields;
use crate::input::{Case, Input, attrs::FieldMode};
use crate::names::{binding, plain};
use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::{Data, Fields, Ident, Index, LitStr, Member, spanned::Spanned as _};

pub fn debug_body(input: &Input<'_>, formatter: &Ident, debug: &Ident) -> TokenStream {
    if let Data::Struct(_) = &input.item.data {
        return case_body(input, &input.cases[0], formatter, debug);
    }

    if input.cases.is_empty() {
        return quote!(match *self {});
    }

    let mut arms = Vec::with_capacity(input.cases.len());
    for case in &input.cases {
        arms.push(case_body(input, case, formatter, debug));
    }

    quote!(match self { #(#arms),* })
}

pub fn case_body(
    input: &Input<'_>,
    case: &Case<'_>,
    formatter: &Ident,
    debug: &Ident,
) -> TokenStream {
    let body = match &case.format {
        Some(format) => {
            let literal = &format.literal;
            let arguments = format
                .arguments
                .iter()
                .map(|index| field_value(input, case, *index));
            quote!(#formatter.write_fmt(::core::format_args!(#literal, #(#arguments),*)))
        }
        None => default_body(input, case, formatter, debug),
    };

    let Some(variant) = case.variant else {
        return body;
    };

    let mut patterns = Vec::new();
    for (index, field) in case.fields.iter().enumerate() {
        let used = !field.formats.is_empty() || field.sets_width_or_precision;
        let pattern = if used && field.options.mode == FieldMode::Normal {
            let local = binding(&input.const_names, &format!("__er_{index}"));
            quote!(#local)
        } else {
            quote!(_)
        };
        patterns.push(pattern);
    }

    let target = quote!(Self::#variant);
    let pattern = with_fields(case, target, &patterns);
    quote!(#pattern => { #body })
}

pub fn default_body(
    input: &Input<'_>,
    case: &Case<'_>,
    formatter: &Ident,
    debug: &Ident,
) -> TokenStream {
    let mut lines = Vec::new();
    for (index, field) in case.fields.iter().enumerate() {
        if field.options.mode == FieldMode::Skip {
            continue;
        }

        let value = field_value(input, case, index);
        let reference = quote_spanned!(field.item.ty.span()=> &&#value);
        if let Some(name) = &field.item.ident {
            let label = LitStr::new(&plain(name), name.span());
            lines.push(quote!(#debug.field(#label, #reference);));
        } else {
            lines.push(quote!(#debug.field(#reference);));
        }
    }

    let mut label = plain(&input.item.ident);
    if let Some(variant) = case.variant {
        label.push_str("::");
        label.push_str(&plain(variant));
    }

    let name = case.variant.unwrap_or(&input.item.ident);
    let label = LitStr::new(&label, name.span());
    match case.shape {
        Fields::Unit => quote!(#formatter.write_str(#label)),
        Fields::Named(_) => quote! {
            let mut #debug = #formatter.debug_struct(#label);
            #(#lines)*
            #debug.finish()
        },
        Fields::Unnamed(_) => quote! {
            let mut #debug = #formatter.debug_tuple(#label);
            #(#lines)*
            #debug.finish()
        },
    }
}

pub fn field_value(input: &Input<'_>, case: &Case<'_>, index: usize) -> TokenStream {
    let field = &case.fields[index];
    if field.options.mode == FieldMode::Censor {
        return quote!(__ErCensored);
    }

    if case.variant.is_some() {
        let local = binding(&input.const_names, &format!("__er_{index}"));
        return quote_spanned!(field.item.ty.span()=> *#local);
    }

    let member = match &field.item.ident {
        Some(name) => Member::Named(name.clone()),
        None => Member::Unnamed(Index::from(index)),
    };

    quote_spanned!(field.item.ty.span()=> self.#member)
}
