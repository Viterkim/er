use crate::{
    fields::with_fields,
    generate::constructors::exact_type,
    input::{Input, attrs::IntoField},
    names::{binding, plain},
};
use proc_macro2::{TokenStream, TokenTree};
use quote::{ToTokens as _, quote};
use std::collections::HashSet;
use syn::{Error, Path, Type, parse_quote};

pub fn implementation(input: &Input<'_>, path: &Path) -> syn::Result<TokenStream> {
    let cases: Vec<_> = input
        .cases
        .iter()
        .filter(|case| case.fields.iter().any(|field| field.options.into.is_some()))
        .collect();
    if cases.is_empty() {
        return Ok(TokenStream::new());
    }

    let mut reserved = HashSet::new();
    let item = input.item;
    let mut tokens: Vec<_> = quote!(#item).into_iter().collect();
    while let Some(token) = tokens.pop() {
        match token {
            TokenTree::Ident(ident) => {
                reserved.insert(plain(&ident));
            }
            TokenTree::Group(group) => tokens.extend(group.stream()),
            _ => (),
        }
    }

    let source = binding(&reserved, "__ErSource");
    let tree = binding(&reserved, "__er_tree");
    let report = binding(&reserved, "__er_report");
    let snapshot = binding(&reserved, "__er_snapshot");
    let extra = binding(&reserved, "__er_extra");
    let name = &item.ident;
    let (_, type_generics, _) = item.generics.split_for_impl();
    let mut sources = HashSet::new();
    let mut implementations = Vec::new();

    for case in &cases {
        let mut top = None;
        let mut reports = 0;
        let mut snapshots = 0;
        let mut extra_fields = Vec::new();
        for (index, field) in case.fields.iter().enumerate() {
            match field.options.into {
                Some(IntoField::Top) => {
                    if top.is_some() {
                        return Err(Error::new_spanned(
                            field.item,
                            "choose one `into_top` field per struct or enum variant",
                        ));
                    }
                    top = Some(field);
                }
                Some(IntoField::ReportString) => reports += 1,
                Some(IntoField::Snapshot) => snapshots += 1,
                None => extra_fields.push((index, field)),
            }
        }

        if reports + snapshots == 0 {
            return Err(Error::new_spanned(
                case.shape,
                "`.er_into()` needs an `into_report_string` or `into_snapshot` field to save diagnostics",
            ));
        }

        let mut generics = item.generics.clone();
        let ty: Type = match top {
            Some(field) => {
                let ty = &field.item.ty;
                if !sources.insert(ty.to_token_stream().to_string()) {
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

        let mut extra_types = Vec::new();
        let mut extra_bindings = Vec::new();
        for (index, field) in &extra_fields {
            let field_type = &field.item.ty;
            let argument = binding(&reserved, &format!("__ErIntoInput{index}"));
            let value = binding(&reserved, &format!("__er_into_value{index}"));

            if field.options.exact || exact_type(field_type, &input.type_names) {
                extra_types.push(quote!(#field_type));
            } else {
                generics.params.push(parse_quote!(#argument));
                generics.make_where_clause().predicates.push(parse_quote!(
                    #argument: ::core::convert::Into<#field_type>
                ));
                extra_types.push(quote!(#argument));
            }

            extra_bindings.push(value);
        }

        let payload = match extra_types.as_slice() {
            [] => quote!(()),
            [ty] => quote!(#ty),
            _ => quote!((#(#extra_types),*)),
        };
        let pattern = match extra_bindings.as_slice() {
            [] => quote!(()),
            [value] => quote!(#value),
            _ => quote!((#(#extra_bindings),*)),
        };
        let (impl_generics, _, where_clause) = generics.split_for_impl();

        let save_report = (reports > 0).then(|| {
            quote! {
                let #report = #tree.er_report_string();
            }
        });
        let save_snapshot = (snapshots > 0).then(|| {
            quote! {
                let #snapshot = #tree.er_snapshot();
            }
        });
        let mut values = Vec::with_capacity(case.fields.len());
        for (index, field) in case.fields.iter().enumerate() {
            let value = match field.options.into {
                Some(IntoField::Top) => quote!(#tree.top),
                Some(IntoField::ReportString) => {
                    reports -= 1;
                    if reports == 0 {
                        quote!(#report)
                    } else {
                        quote!(::core::clone::Clone::clone(&#report))
                    }
                }
                Some(IntoField::Snapshot) => {
                    snapshots -= 1;
                    if snapshots == 0 {
                        quote!(#snapshot)
                    } else {
                        quote!(::core::clone::Clone::clone(&#snapshot))
                    }
                }
                None => {
                    let value = binding(&reserved, &format!("__er_into_value{index}"));
                    if field.options.exact || exact_type(&field.item.ty, &input.type_names) {
                        quote!(#value)
                    } else {
                        quote!(::core::convert::Into::into(#value))
                    }
                }
            };
            values.push(value);
        }
        let target = match case.variant {
            Some(variant) => quote!(Self::#variant),
            None => quote!(Self),
        };
        let body = with_fields(case, target, &values);
        implementations.push(quote! {
            impl #impl_generics #path::ErFromTree<#ty, #payload> for #name #type_generics #where_clause {
                fn er_from_tree(#tree: #path::ErTree<#ty>, #extra: #payload) -> Self {
                    let #pattern = #extra;
                    #save_report
                    #save_snapshot
                    #body
                }
            }
        });
    }

    Ok(quote!(#(#implementations)*))
}
