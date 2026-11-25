pub mod impls;

use super::{
    Input,
    attrs::FieldMode,
    format::{FormatBody, FormatTrait, Output},
};
use crate::fields::option_inner;
use crate::names::plain;
use quote::ToTokens as _;
use std::collections::HashSet;
use syn::{Generics, Member, Type, parse_quote, visit::Visit as _};

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

/// Lifetimes keep their `'` in the parameter names. Without it `'io` matched the module in `io::Error`, and a
/// concrete field got bounds like it was generic.
pub fn lifetime_name(lifetime: &syn::Lifetime) -> String {
    format!("'{}", plain(&lifetime.ident))
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
            .map(|param| lifetime_name(&param.lifetime)),
    );

    let mut generics = input.item.generics.clone();
    if parameters.is_empty() {
        return generics;
    }

    let mut seen = HashSet::new();
    let mut seen_fields = HashSet::new();
    let name = plain(&input.item.ident);

    for case in &input.cases {
        let mut requirements = Vec::new();
        for output in [Output::Display, Output::DisplayAlt, Output::Debug] {
            if matches!(output, Output::DisplayAlt) && case.formats.display_alt.is_none() {
                continue;
            }

            let tried = case.formats.tried(output);

            for body in &tried.formats {
                let FormatBody::Template(format) = body else {
                    continue;
                };

                for (argument, requirement) in &format.requirements {
                    let field = &case.fields[argument.field];
                    let ty = match option_inner(&field.item.ty) {
                        Some(inner) if argument.optional => inner,
                        Some(_) | None => &field.item.ty,
                    };

                    if let Some(ty) = member_type(ty, &argument.members) {
                        requirements.push((field, ty, *requirement));
                    }
                }
            }

            if tried.default {
                requirements.extend(
                    case.fields
                        .iter()
                        .filter(|field| field.printed_by_default())
                        .map(|field| (field, &field.item.ty, FormatTrait::Debug)),
                );
            }
        }

        for (field, ty, format) in requirements {
            if field.options.mode != FieldMode::Normal {
                continue;
            }

            if !seen_fields.insert((ty as *const Type, format)) {
                continue;
            }

            let mut types = Vec::new();
            let mut bounds = FormatBounds::new(&name, &parameters, &mut types, false);
            bounds.visit_type(ty);

            for ty in types {
                let key = bound_key(ty);
                if key
                    .as_ref()
                    .is_none_or(|key| seen.insert((key.clone(), format)))
                {
                    let format = format.ident();
                    let predicate = parse_quote!(#ty: ::core::fmt::#format);
                    generics.make_where_clause().predicates.push(predicate);
                }
            }
        }
    }

    generics
}

// Named types hide their field types from the derive. Rust checks those accesses.
pub fn member_type<'a>(mut ty: &'a Type, members: &[Member]) -> Option<&'a Type> {
    for member in members {
        loop {
            ty = match ty {
                Type::Reference(reference) => &reference.elem,
                Type::Paren(paren) => &paren.elem,
                Type::Group(group) => &group.elem,
                _ => break,
            };
        }

        let (Type::Tuple(tuple), Member::Unnamed(index)) = (ty, member) else {
            return None;
        };
        ty = tuple.elems.iter().nth(index.index as usize)?;
    }

    Some(ty)
}
