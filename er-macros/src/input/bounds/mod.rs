use super::{Input, attrs::FieldMode};
use crate::names::plain;
use quote::{ToTokens as _, quote};
use std::collections::HashSet;
use syn::{Generics, Type, visit::Visit as _};

pub mod impls;

pub struct FormatBounds<'a, 'out> {
    pub name: &'a str,
    pub parameters: &'a HashSet<String>,
    pub types: &'out mut Vec<&'a Type>,
    pub uses_parameters: bool,
    pub recursive: bool,
    pub keep_type_bound: bool,
}

pub fn format_generics(input: &Input<'_>) -> syn::Result<Generics> {
    let mut parameters: HashSet<_> = input
        .type_names
        .union(&input.const_names)
        .cloned()
        .collect();
    parameters.extend(
        input
            .item
            .generics
            .lifetimes()
            .map(|param| plain(&param.lifetime.ident)),
    );

    let mut generics = input.item.generics.clone();
    if parameters.is_empty() {
        return Ok(generics);
    }

    let mut seen = HashSet::new();
    let name = plain(&input.item.ident);
    for case in &input.cases {
        for field in &case.fields {
            let formats = &field.formats;
            if field.options.mode != FieldMode::Normal || formats.is_empty() {
                continue;
            }

            let mut types = Vec::new();
            let mut bounds = FormatBounds::new(&name, &parameters, &mut types, false);
            bounds.visit_type(&field.item.ty);

            for ty in types {
                let key = ty.to_token_stream().to_string();
                for format in formats {
                    if seen.insert((key.clone(), *format)) {
                        let format = format.ident();
                        let predicate = syn::parse2(quote!(
                            #ty: ::core::fmt::#format
                        ))?;
                        generics.make_where_clause().predicates.push(predicate);
                    }
                }
            }
        }
    }

    Ok(generics)
}
