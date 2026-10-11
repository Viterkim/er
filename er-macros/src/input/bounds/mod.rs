pub mod impls;

use super::{
    Input,
    attrs::{FieldMode, IntoField},
};
use crate::names::plain;
use quote::ToTokens as _;
use std::collections::HashSet;
use syn::{Generics, Type, parse_quote, visit::Visit as _};

pub struct FormatBounds<'a, 'out> {
    pub name: &'a str,
    pub parameters: &'a HashSet<String>,
    pub types: &'out mut Vec<&'a Type>,
    pub uses_parameters: bool,
    pub recursive: bool,
    pub keep_type_bound: bool,
}

pub struct BoundSyntax {
    pub opaque: bool,
}

// Identical macro calls can expand to different types at different locations.
pub fn bound_key(ty: &Type) -> Option<String> {
    let mut syntax = BoundSyntax { opaque: false };
    syntax.visit_type(ty);

    (!syntax.opaque).then(|| ty.to_token_stream().to_string())
}

pub fn format_generics(input: &Input<'_>) -> Generics {
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
        return generics;
    }

    let mut seen = HashSet::new();
    let name = plain(&input.item.ident);
    for case in &input.cases {
        for field in &case.fields {
            let formats = &field.formats;
            if field.options.mode != FieldMode::Normal
                || formats.is_empty()
                || field.options.into == Some(IntoField::Snapshot)
            {
                continue;
            }

            let mut types = Vec::new();
            let mut bounds = FormatBounds::new(&name, &parameters, &mut types, false);
            bounds.visit_type(&field.item.ty);

            for ty in types {
                let key = bound_key(ty);
                for format in formats {
                    if key
                        .as_ref()
                        .is_none_or(|key| seen.insert((key.clone(), *format)))
                    {
                        let format = format.ident();
                        let predicate = parse_quote!(
                            #ty: ::core::fmt::#format
                        );
                        generics.make_where_clause().predicates.push(predicate);
                    }
                }
            }
        }
    }

    generics
}
