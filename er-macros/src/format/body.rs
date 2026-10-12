use crate::fields::with_fields;
use crate::input::{
    Case, Input,
    attrs::{FieldMode, IntoField},
    format::{FormatBody, Output},
};
use crate::names::{binding, plain};
use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::{Data, Fields, Ident, Index, LitStr, Member, spanned::Spanned as _};

pub fn body(input: &Input<'_>, output: Output, formatter: &Ident, debug: &Ident) -> TokenStream {
    if let Data::Struct(_) = &input.item.data {
        return case_body(input, &input.cases[0], output, formatter, debug);
    }

    if input.cases.is_empty() {
        return quote!(match *self {});
    }

    let mut arms = Vec::with_capacity(input.cases.len());
    for case in &input.cases {
        arms.push(case_body(input, case, output, formatter, debug));
    }

    quote!(match self { #(#arms),* })
}

pub fn case_body(
    input: &Input<'_>,
    case: &Case<'_>,
    output: Output,
    formatter: &Ident,
    debug: &Ident,
) -> TokenStream {
    let body = match case.formats.get(output) {
        Some(FormatBody::Template(format)) => {
            let literal = &format.literal;
            let arguments = format.arguments.iter().map(|argument| {
                let value = field_value(input, case, argument.field);
                if argument.members.is_empty()
                    || case.fields[argument.field].options.mode == FieldMode::Censor
                {
                    value
                } else {
                    let members = &argument.members;
                    quote_spanned!(case.fields[argument.field].item.ty.span()=> (#value)#(.#members)*)
                }
            });
            quote!(#formatter.write_fmt(::core::format_args!(#literal, #(#arguments),*)))
        }
        Some(FormatBody::Function(path)) => quote!(#path(self, #formatter)),
        None => default_body(input, case, formatter, debug),
    };

    let Some(variant) = case.variant else {
        return body;
    };

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

    let value = if case.variant.is_some() {
        let local = binding(&input.names, &format!("__er_{index}"));
        quote_spanned!(field.item.ty.span()=> *#local)
    } else {
        let member = match &field.item.ident {
            Some(name) => Member::Named(name.clone()),
            None => Member::Unnamed(Index::from(index)),
        };
        quote_spanned!(field.item.ty.span()=> self.#member)
    };

    if field.options.into == Some(IntoField::Snapshot) {
        quote_spanned!(field.item.ty.span()=> (#value).er_report())
    } else {
        value
    }
}
