pub mod impls;

use super::{
    Input,
    attrs::FieldMode,
    format::{FormatBody, FormatTrait},
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

// Keep `'io` distinct from the module in `io::Error`.
pub fn lifetime_name(lifetime: &syn::Lifetime) -> String {
    format!("'{}", plain(&lifetime.ident))
}

pub fn format_generics(input: &Input<'_>) -> Generics {
    let mut generics = input.item.generics.clone();
    if input.parameters.is_empty() {
        return generics;
    }

    let mut seen = HashSet::new();
    let mut seen_fields = HashSet::new();
    let name = plain(&input.item.ident);

    for case in &input.cases {
        let mut requirements = Vec::new();
        let mut used_default = false;
        for &output in case.formats.outputs() {
            let fallbacks = case.formats.fallbacks(output);
            for body in fallbacks.formats.into_iter().flatten() {
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

            if fallbacks.use_default && !used_default {
                used_default = true;
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
            let mut bounds = FormatBounds::new(&name, &input.parameters, &mut types, false);
            bounds.visit_type(ty);

            for ty in types {
                if bound_key(ty).is_none_or(|key| seen.insert((key, format))) {
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
