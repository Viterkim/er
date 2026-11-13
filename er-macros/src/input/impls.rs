use super::{
    Case, Field, IdentifierNames, Input, ValueNames, attrs,
    attrs::FieldMode,
    format::{FormatBody, FormatTrait, Formats, Output},
};
use crate::names::{identifiers, plain};
use proc_macro2::TokenStream;
use std::collections::HashSet;
use syn::{Data, DeriveInput, Ident, Type, visit::Visit as _};

impl Field<'_> {
    pub fn printed_by_default(&self) -> bool {
        match self.options.mode {
            FieldMode::Skip => false,
            FieldMode::Normal | FieldMode::Censor => self
                .options
                .source
                .is_none_or(|source| source.show_in_parent),
        }
    }
}

impl<'a> Case<'a> {
    pub fn parse(
        variant: Option<&'a Ident>,
        attrs: &'a [syn::Attribute],
        shape: &'a syn::Fields,
        format: &attrs::FormatOptions,
        type_names: &HashSet<String>,
        no_constructors: bool,
    ) -> syn::Result<Self> {
        let mut fields = Vec::with_capacity(shape.len());
        for item in shape {
            let mut options = attrs::field(&item.attrs)?;
            // Both get filled in by the constructors. Without them the field just never gets that value, and
            // nothing would say so. A leftover `exact` changes nothing either way.
            if no_constructors && (options.default.is_some() || options.src_location) {
                return Err(syn::Error::new_spanned(
                    item,
                    "`default` and `src_location` get filled in by the constructors, and `no_constructors` turned them off",
                ));
            }
            options.exact |= exact_type(&item.ty, type_names);
            if options.source.is_some() && options.mode == FieldMode::Censor {
                return Err(syn::Error::new_spanned(
                    item,
                    "`source` exposes this error to reporters. Remove `source` to keep the field censored",
                ));
            }
            if options.source.is_some()
                && fields
                    .iter()
                    .any(|field: &Field<'_>| field.options.source.is_some())
            {
                return Err(syn::Error::new_spanned(
                    item,
                    "only one field can be the `source`",
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
            && let Some(field) = fields.iter().find(|field| field.options.default.is_some())
        {
            return Err(syn::Error::new_spanned(
                field.item,
                "`default` only works on struct fields, variant constructors take every field",
            ));
        }

        let formats = Formats::parse(format, &fields)?;
        for output in [Output::Display, Output::DisplayAlt, Output::Debug] {
            if matches!(output, Output::DisplayAlt) && formats.display_alt.is_none() {
                continue;
            }
            let tried = formats.tried(output);

            for body in &tried.formats {
                let FormatBody::Template(format) = body else {
                    continue;
                };

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

            if tried.default {
                for field in &mut fields {
                    if field.printed_by_default() && !field.formats.contains(&FormatTrait::Debug) {
                        field.formats.push(FormatTrait::Debug);
                    }
                }
            }
        }

        Ok(Self {
            variant,
            attrs,
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
                &[],
                &data.fields,
                &options.format,
                &type_names,
                options.no_constructors,
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
                    let case = Case::parse(
                        Some(&variant.ident),
                        &variant.attrs,
                        &variant.fields,
                        &format,
                        &type_names,
                        options.no_constructors,
                    )?;
                    cases.push(case);
                }

                cases
            }
            Data::Union(data) => {
                return Err(syn::Error::new_spanned(
                    data.union_token,
                    "Er works on structs and enums, not unions",
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
                .params
                .iter()
                .filter_map(|parameter| match parameter {
                    syn::GenericParam::Type(parameter) => Some(("type", &parameter.ident)),
                    syn::GenericParam::Const(parameter) => Some(("const", &parameter.ident)),
                    syn::GenericParam::Lifetime(_) => None,
                })
                .find(|(_, parameter)| plain(parameter) == name);
            if let Some((kind, parameter)) = parameter {
                let ident = match &wrap.name {
                    Some(ident) => ident,
                    None => parameter,
                };

                return Err(syn::Error::new_spanned(
                    ident,
                    format!(
                        "the Wrap would be called `{name}`, which is already a {kind} parameter here, \
                         pick another with `wrap(name = ...)`"
                    ),
                ));
            }
        }

        let mut names = IdentifierNames::default();
        names.visit_derive_input(item);

        // Default expressions and format functions don't run inside the constructors, only the field types share
        // a scope with the arguments.
        let mut scope_names = ValueNames::default();
        scope_names.visit_generics(&item.generics);
        for field in cases.iter().flat_map(|case| &case.fields) {
            scope_names.visit_type(&field.item.ty);
        }

        Ok(Self {
            item,
            options,
            cases,
            const_names,
            type_names,
            names: names.names,
            scope_names: scope_names.names,
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

impl<'a> syn::visit::Visit<'a> for ValueNames {
    fn visit_expr_path(&mut self, path: &'a syn::ExprPath) {
        if let Some(ident) = path.path.get_ident() {
            self.names.insert(plain(ident));
        }

        syn::visit::visit_expr_path(self, path);
    }

    fn visit_const_param(&mut self, param: &'a syn::ConstParam) {
        self.names.insert(plain(&param.ident));
        syn::visit::visit_const_param(self, param);
    }

    // A path in a generic argument parses as a type, `Buf<SIZE>` could be a const all the same.
    fn visit_type_path(&mut self, path: &'a syn::TypePath) {
        if path.qself.is_none()
            && let Some(ident) = path.path.get_ident()
        {
            self.names.insert(plain(ident));
        }

        syn::visit::visit_type_path(self, path);
    }

    // We can't see what a macro expands to, so every name in it counts.
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
