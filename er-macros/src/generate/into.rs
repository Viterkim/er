use super::construct::{field_inputs, tuple_input};
use crate::{
    fields::with_fields,
    input::{Case, Field, Input, attrs::IntoField, bounds::bound_key},
    names::{binding, shield_bindings},
};
use proc_macro2::TokenStream;
use quote::quote;
use std::collections::HashSet;
use syn::{Error, Ident, Path, Type, parse_quote};

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
            None => fields.extra.push((index, field)),
        }
    }

    if fields.last_report.is_none()
        && fields.last_single_line.is_none()
        && fields.last_snapshot.is_none()
    {
        return Err(Error::new_spanned(
            case.shape,
            "`.er_into()` needs an `into_report_string` or `into_snapshot` field to save diagnostics",
        ));
    }

    Ok(fields)
}

pub fn implementation(input: &Input<'_>, path: &Path) -> syn::Result<TokenStream> {
    let cases: Vec<_> = input
        .cases
        .iter()
        .filter(|case| case.fields.iter().any(|field| field.options.into.is_some()))
        .collect();
    if cases.is_empty() {
        return Ok(TokenStream::new());
    }

    let item = input.item;
    let reserved = &input.names;
    let source = binding(reserved, "__ErSource");
    let tree = binding(reserved, "__er_tree");
    let report = binding(reserved, "__er_report");
    let saved_report = binding(reserved, "__er_saved_report");
    let single_line = binding(reserved, "__er_single_line");
    let snapshot = binding(reserved, "__er_snapshot");
    let extra = binding(reserved, "__er_extra");
    let name = &item.ident;
    let (_, type_generics, _) = item.generics.split_for_impl();
    let mut sources = HashSet::new();
    let mut implementations = Vec::new();
    let mut bindings = vec![tree.clone(), report.clone(), extra.clone()];

    for case in &cases {
        let fields = converted_fields(case)?;
        if fields.last_report.is_some() {
            bindings.push(saved_report.clone());
        }
        if fields.last_single_line.is_some() {
            bindings.push(single_line.clone());
        }
        if fields.last_snapshot.is_some() {
            bindings.push(snapshot.clone());
        }
        let mut generics = item.generics.clone();
        let ty: Type = match fields.top {
            Some(field) => {
                let ty = &field.item.ty;
                if bound_key(ty).is_some_and(|key| !sources.insert(key)) {
                    return Err(Error::new_spanned(
                        field.item,
                        "these variants accept the same top type, so `.er_into()` cannot choose between them",
                    ));
                }
                ty.clone()
            }
            None => {
                if cases.len() > 1 {
                    return Err(Error::new_spanned(
                        case.shape,
                        "multiple converted variants need distinct `into_top` types so `.er_into()` can choose one",
                    ));
                }
                generics.params.push(parse_quote!(#source));
                parse_quote!(#source)
            }
        };
        generics.make_where_clause().predicates.push(parse_quote!(
            #ty: ::core::error::Error + 'static
        ));

        let inputs = field_inputs(fields.extra.iter().copied(), &mut generics, reserved);
        bindings.extend(inputs.bindings.iter().cloned());
        let payload = tuple_input(&inputs.types);
        let pattern = tuple_input(&inputs.bindings);
        let (impl_generics, _, where_clause) = generics.split_for_impl();
        let mut values = vec![TokenStream::new(); case.fields.len()];
        for ((index, _), value) in fields.extra.iter().zip(inputs.values) {
            values[*index] = value;
        }

        for (index, field) in case.fields.iter().enumerate() {
            values[index] = match field.options.into {
                Some(IntoField::Top) => quote!(#report.tree.top),
                Some(IntoField::ReportString { single_line: false }) => {
                    saved_value(&saved_report, fields.last_report == Some(index))
                }
                Some(IntoField::ReportString { single_line: true }) => {
                    saved_value(&single_line, fields.last_single_line == Some(index))
                }
                Some(IntoField::Snapshot) => {
                    saved_value(&snapshot, fields.last_snapshot == Some(index))
                }
                None => continue,
            };
        }

        let save_report = fields.last_report.map(|_| {
            quote! {
                let #saved_report = #report.tree.er_report().layout(#report.layout).er_report_string();
            }
        });
        let save_single_line = fields.last_single_line.map(|_| {
            let value = if fields.last_report.is_some() {
                quote! {
                    match #report.layout {
                        #path::Layout::SingleLine => ::core::clone::Clone::clone(&#saved_report),
                        #path::Layout::Multiline => #report.tree.er_report().single_line().er_report_string(),
                    }
                }
            } else {
                quote!(#report.tree.er_report().single_line().er_report_string())
            };

            quote!(let #single_line = #value;)
        });
        let save_snapshot = fields
            .last_snapshot
            .map(|_| quote!(let #snapshot = #report.tree.er_snapshot();));
        let target = match case.variant {
            Some(variant) => quote!(Self::#variant),
            None => quote!(Self),
        };
        let body = with_fields(case, target, &values);

        implementations.push(quote! {
            impl #impl_generics #path::ErFromTree<#ty, #payload> for #name #type_generics #where_clause {
                fn er_from_tree(#tree: #path::ErTree<#ty>, #extra: #payload) -> Self {
                    <Self as #path::ErFromTree<#ty, #payload>>::er_from_report(#tree.into_er_report(), #extra)
                }

                fn er_from_report(#report: #path::ErReport<#ty>, #extra: #payload) -> Self {
                    let #pattern = #extra;
                    #save_report
                    #save_single_line
                    #save_snapshot
                    #body
                }
            }
        });
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
