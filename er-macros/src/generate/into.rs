use super::construct::{caller_tracking, default_value, field_inputs, tuple_input};
use crate::{
    fields::with_fields,
    input::{Case, Field, Input, attrs::IntoField, bounds::bound_key},
    names::{binding, shield_bindings},
};
use proc_macro2::TokenStream;
use quote::quote;
use std::collections::HashSet;
use syn::{Error, Ident};

pub struct ConvertedFields<'a, 'item> {
    pub top: Option<&'a Field<'item>>,
    pub last_report: Option<usize>,
    pub last_single_line: Option<usize>,
    pub last_snapshot: Option<usize>,
    pub extra: Vec<(usize, &'a Field<'item>)>,
}

pub fn converted_fields<'a, 'item>(
    case: &'a Case<'item>,
) -> syn::Result<ConvertedFields<'a, 'item>> {
    let mut fields = ConvertedFields {
        top: None,
        last_report: None,
        last_single_line: None,
        last_snapshot: None,
        extra: Vec::new(),
    };

    for (index, field) in case.fields.iter().enumerate() {
        match field.options.into {
            Some(IntoField::Top) => {
                if fields.top.is_some() {
                    return Err(Error::new_spanned(
                        field.item,
                        "choose one `into_top` field per struct or enum variant",
                    ));
                }
                fields.top = Some(field);
            }
            Some(IntoField::ReportString { single_line: false }) => {
                fields.last_report = Some(index)
            }
            Some(IntoField::ReportString { single_line: true }) => {
                fields.last_single_line = Some(index)
            }
            Some(IntoField::Snapshot) => fields.last_snapshot = Some(index),
            None if !field.options.src_location => fields.extra.push((index, field)),
            None => (),
        }
    }

    if fields.last_report.is_none()
        && fields.last_single_line.is_none()
        && fields.last_snapshot.is_none()
    {
        return Err(Error::new_spanned(
            case.shape,
            "converted errors need an `into_report_string` or `into_snapshot` field to save diagnostics",
        ));
    }

    Ok(fields)
}

pub fn implementation(input: &Input<'_>) -> syn::Result<TokenStream> {
    let cases: Vec<_> = input
        .cases
        .iter()
        .filter(|case| case.fields.iter().any(|field| field.options.into.is_some()))
        .collect();
    let case_count = cases.len();
    let item = input.item;
    let name = &item.ident;
    let (_, type_generics, _) = item.generics.split_for_impl();
    let reserved = &input.names;
    let mut sources = HashSet::new();
    let mut implementations = Vec::new();
    let mut bindings = Vec::new();

    for case in cases {
        let fields = converted_fields(case)?;
        if let Some(field) = fields.top {
            if bound_key(&field.item.ty).is_some_and(|key| !sources.insert(key)) {
                return Err(Error::new_spanned(
                    field.item,
                    "these converted variants accept the same top type",
                ));
            }
        } else if case_count > 1 {
            return Err(Error::new_spanned(
                case.shape,
                "multiple converted variants need distinct `into_top` types",
            ));
        }

        // These standard tuple shapes must match the report's saved fields across versions.
        let tag = 2
            + usize::from(fields.top.is_some())
            + 2 * usize::from(fields.last_report.is_some())
            + 4 * usize::from(fields.last_single_line.is_some())
            + 8 * usize::from(fields.last_snapshot.is_some());
        let mut saved_types = Vec::new();
        let mut saved_bindings = Vec::new();
        let top = binding(reserved, "__er_top");
        let report = binding(reserved, "__er_saved_report");
        let single = binding(reserved, "__er_single_line");
        let snapshot = binding(reserved, "__er_snapshot");
        for (field, saved) in [
            (fields.top, &top),
            (fields.last_report.map(|index| &case.fields[index]), &report),
            (
                fields.last_single_line.map(|index| &case.fields[index]),
                &single,
            ),
            (
                fields.last_snapshot.map(|index| &case.fields[index]),
                &snapshot,
            ),
        ] {
            if let Some(field) = field {
                saved_types.push(&field.item.ty);
                saved_bindings.push(saved);
                bindings.push(saved.clone());
            }
        }
        let projection = quote!((#(#saved_types,)*));
        let saved_pattern = quote!((#(#saved_bindings,)*));
        let target = match case.variant {
            Some(variant) => quote!(Self::#variant),
            None => quote!(Self),
        };

        for defaulted in [false, true] {
            if defaulted
                && !fields
                    .extra
                    .iter()
                    .any(|(_, field)| field.options.default.is_some())
            {
                continue;
            }
            let mut generics = item.generics.clone();
            let selected: Vec<_> = fields
                .extra
                .iter()
                .copied()
                .filter(|(_, field)| !defaulted || field.options.default.is_none())
                .collect();
            let inputs = field_inputs(selected.iter().copied(), &mut generics, reserved);
            bindings.extend(inputs.bindings.iter().cloned());
            let mut payload = tuple_input(&inputs.types);
            let mut pattern = tuple_input(&inputs.bindings);
            if defaulted && !selected.is_empty() {
                payload = quote!(((#payload, [(); 1]),));
                pattern = quote!(((#pattern, _),));
            }
            let (source, pattern) = if selected.is_empty() {
                (
                    quote!((#projection, [(); #tag])),
                    quote!((#saved_pattern, _)),
                )
            } else {
                (
                    quote!(((#projection, #payload), [(); #tag])),
                    quote!(((#saved_pattern, #pattern), _)),
                )
            };
            let mut values = vec![TokenStream::new(); case.fields.len()];
            for ((index, _), value) in selected.iter().zip(inputs.values) {
                values[*index] = value;
            }
            for (index, field) in case.fields.iter().enumerate() {
                if field.options.src_location {
                    values[index] = quote!(::core::panic::Location::caller());
                    continue;
                }

                values[index] = match field.options.into {
                    Some(IntoField::Top) => quote!(#top),
                    Some(IntoField::ReportString { single_line: false }) => {
                        saved_value(&report, fields.last_report == Some(index))
                    }
                    Some(IntoField::ReportString { single_line: true }) => {
                        saved_value(&single, fields.last_single_line == Some(index))
                    }
                    Some(IntoField::Snapshot) => {
                        saved_value(&snapshot, fields.last_snapshot == Some(index))
                    }
                    None if defaulted => {
                        default_value(field, &mut generics).unwrap_or_else(|| values[index].clone())
                    }
                    None => continue,
                };
            }
            let tracking = caller_tracking(case);
            let body = with_fields(case, target.clone(), &values);
            let (impl_generics, _, where_clause) = generics.split_for_impl();
            implementations.push(quote! {
                impl #impl_generics ::core::convert::From<#source> for #name #type_generics #where_clause {
                    #tracking
                    fn from(#pattern: #source) -> Self {
                        #body
                    }
                }
            });
        }
    }

    let shields = shield_bindings(&bindings);
    Ok(quote! {
        const _: () = {
            #shields
            #(#implementations)*
        };
    })
}

pub fn saved_value(name: &Ident, last: bool) -> TokenStream {
    if last {
        quote!(#name)
    } else {
        quote!(::core::clone::Clone::clone(&#name))
    }
}
