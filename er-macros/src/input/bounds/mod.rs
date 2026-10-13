pub mod impls;

use super::{
    Input,
    attrs::{FieldMode, IntoField},
    format::{FormatBody, FormatTrait, Output},
};
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
    let mut seen_fields = HashSet::new();
    let name = plain(&input.item.ident);

    for case in &input.cases {
        let mut requirements = Vec::new();
        for output in [Output::Display, Output::DisplayAlt, Output::Debug] {
            if matches!(output, Output::DisplayAlt) && case.formats.display_alt.is_none() {
                continue;
            }

            match case.formats.get(output) {
                Some(FormatBody::Template(format)) => {
                    for (argument, requirement) in &format.requirements {
                        let field = &case.fields[argument.field];
                        if let Some(ty) = member_type(&field.item.ty, &argument.members) {
                            requirements.push((field, ty, *requirement));
                        }
                    }
                }
                Some(FormatBody::Function(_)) => (),
                None => {
                    requirements.extend(
                        case.fields
                            .iter()
                            .map(|field| (field, &field.item.ty, FormatTrait::Debug)),
                    );
                }
            }
        }

        for (field, ty, format) in requirements {
            if field.options.mode != FieldMode::Normal
                || field.options.into == Some(IntoField::Snapshot)
            {
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
