use super::{
    Case, Field, IdentifierNames, Input, attrs,
    attrs::FieldMode,
    format::{FormatBody, FormatTrait, Formats, Output},
};
use crate::names::{identifiers, plain};
use proc_macro2::TokenStream;
use std::collections::HashSet;
use syn::{Data, DeriveInput, Ident, Type, visit::Visit as _};

impl<'a> Case<'a> {
    pub fn parse(
        variant: Option<&'a Ident>,
        shape: &'a syn::Fields,
        format: &attrs::FormatOptions,
        type_names: &HashSet<String>,
    ) -> syn::Result<Self> {
        let mut fields = Vec::with_capacity(shape.len());
        for item in shape {
            let mut options = attrs::field(&item.attrs)?;
            options.exact |= exact_type(&item.ty, type_names);
            if options.source.is_some()
                && fields
                    .iter()
                    .any(|field: &Field<'_>| field.options.source.is_some())
            {
                return Err(syn::Error::new_spanned(
                    item,
                    "choose one source field per struct or enum variant",
                ));
            }
            fields.push(Field {
                item,
                options,
                formats: Vec::new(),
                used_in_format: false,
            });
        }

        if variant.is_some()
            && !fields.iter().any(|field| field.options.into.is_some())
            && let Some(field) = fields.iter().find(|field| field.options.default.is_some())
        {
            return Err(syn::Error::new_spanned(
                field.item,
                "default fields on enum variants need an `into_*` conversion; variant constructors take every field",
            ));
        }

        let formats = Formats::parse(format, &fields)?;
        for output in [Output::Display, Output::DisplayAlt, Output::Debug] {
            if matches!(output, Output::DisplayAlt) && formats.display_alt.is_none() {
                continue;
            }
            match formats.get(output) {
                Some(FormatBody::Template(format)) => {
                    for (argument, requirement) in &format.requirements {
                        let index = argument.field;
                        if !fields[index].formats.contains(requirement) {
                            fields[index].formats.push(*requirement);
                        }
                    }
                    for argument in &format.arguments {
                        fields[argument.field].used_in_format = true;
                    }
                }
                Some(FormatBody::Function(_)) => (),
                None => {
                    for field in &mut fields {
                        if field.options.mode != FieldMode::Skip
                            && !field.formats.contains(&FormatTrait::Debug)
                        {
                            field.formats.push(FormatTrait::Debug);
                        }
                    }
                }
            }
        }

        Ok(Self {
            variant,
            shape,
            fields,
            formats,
        })
    }
}

impl<'a> Input<'a> {
    pub fn parse(item: &'a DeriveInput) -> syn::Result<Self> {
        let options = attrs::container(&item.attrs)?;
        if let Some(wrap) = &options.wrap
            && let Some(name) = &wrap.name
            && plain(name) == plain(&item.ident)
        {
            return Err(syn::Error::new_spanned(
                name,
                "the wrapper needs a different name from the error type",
            ));
        }

        let type_names: HashSet<_> = item
            .generics
            .type_params()
            .map(|param| plain(&param.ident))
            .collect();

        let cases = match &item.data {
            Data::Struct(data) => vec![Case::parse(
                None,
                &data.fields,
                &options.format,
                &type_names,
            )?],
            Data::Enum(data) => {
                if let Some(attrs::FormatValue::Literal(literal)) = &options.format.shared {
                    return Err(syn::Error::new_spanned(
                        literal,
                        "put `format = \"...\"` on each enum variant that needs it",
                    ));
                }

                let mut cases = Vec::with_capacity(data.variants.len());
                for variant in &data.variants {
                    let mut format = attrs::variant(&variant.attrs)?;
                    if format.shared.is_none() {
                        format.display = format.display.or_else(|| options.format.display.clone());
                        format.debug = format.debug.or_else(|| options.format.debug.clone());
                        format.shared = options.format.shared.clone();
                    }
                    format.display_alt = format
                        .display_alt
                        .or_else(|| options.format.display_alt.clone());
                    let case =
                        Case::parse(Some(&variant.ident), &variant.fields, &format, &type_names)?;
                    cases.push(case);
                }

                cases
            }
            Data::Union(data) => {
                return Err(syn::Error::new_spanned(
                    data.union_token,
                    "Er supports structs and enums only",
                ));
            }
        };

        let const_names = item
            .generics
            .const_params()
            .map(|param| plain(&param.ident))
            .collect();
        if let Some(wrap) = &options.wrap {
            let name = match &wrap.name {
                Some(name) => plain(name),
                None => format!("{}Wrap", plain(&item.ident)),
            };

            let parameter = item
                .generics
                .type_params()
                .find(|parameter| plain(&parameter.ident) == name);
            if let Some(parameter) = parameter {
                let ident = match &wrap.name {
                    Some(ident) => ident,
                    None => &parameter.ident,
                };

                return Err(syn::Error::new_spanned(
                    ident,
                    format!(
                        "wrapper name `{name}` clashes with a type parameter; \
                         use `wrap(name = ...)` to choose another name"
                    ),
                ));
            }
        }

        let mut names = IdentifierNames::default();
        names.visit_derive_input(item);

        Ok(Self {
            item,
            options,
            cases,
            const_names,
            type_names,
            names: names.names,
        })
    }
}

impl<'a> syn::visit::Visit<'a> for IdentifierNames {
    fn visit_ident(&mut self, ident: &'a Ident) {
        self.names.insert(plain(ident));
    }

    fn visit_token_stream(&mut self, tokens: &'a TokenStream) {
        self.names.extend(identifiers(tokens.clone()));
    }
}

pub fn exact_type(ty: &Type, type_names: &HashSet<String>) -> bool {
    match ty {
        Type::Paren(ty) => return exact_type(&ty.elem, type_names),
        Type::Group(ty) => return exact_type(&ty.elem, type_names),
        _ => (),
    }

    if matches!(
        ty,
        Type::FnPtr(_)
            | Type::Reference(_)
            | Type::Ptr(_)
            | Type::Tuple(_)
            | Type::Array(_)
            | Type::Slice(_)
    ) {
        return true;
    }

    let Type::Path(path) = ty else {
        return false;
    };
    if path.qself.is_some() {
        return false;
    }
    if let Some(ident) = path.path.get_ident()
        && type_names.contains(&plain(ident))
    {
        return true;
    }

    let segments = &path.path.segments;
    let ordinary = segments.len() == 1;
    let primitive_path = segments.len() == 3
        && matches!(plain(&segments[0].ident).as_str(), "core" | "std")
        && plain(&segments[1].ident) == "primitive";
    if !ordinary && !primitive_path {
        return false;
    }
    let Some(segment) = segments.last() else {
        return false;
    };

    matches!(
        plain(&segment.ident).as_str(),
        "bool"
            | "char"
            | "i8"
            | "i16"
            | "i32"
            | "i64"
            | "i128"
            | "isize"
            | "u8"
            | "u16"
            | "u32"
            | "u64"
            | "u128"
            | "usize"
            | "f32"
            | "f64"
    )
}
