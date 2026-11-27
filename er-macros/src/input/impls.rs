use super::{
    Case, Constructors, Field, IdentifierNames, Input, ValueNames, attrs,
    attrs::FieldMode,
    bounds::lifetime_name,
    format::{FormatBody, Formats},
};
use crate::names::{collect_identifiers, plain};
use proc_macro2::TokenStream;
use std::collections::HashSet;
use syn::{Data, DeriveInput, Fields, Ident, Type, Variant, Visibility, visit::Visit as _};

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

impl<'a> Constructors<'a> {
    fn for_public_struct(shape: &'a Fields) -> Self {
        let mut restrictions = shape
            .iter()
            .map(|field| &field.vis)
            .filter(|vis| !matches!(vis, Visibility::Public(_)));
        let Some(mut narrowest) = restrictions.next() else {
            return Self::TypeVisibility;
        };

        for other in restrictions {
            let (Visibility::Restricted(a), Visibility::Restricted(b)) = (narrowest, other) else {
                return Self::Restricted(&Visibility::Inherited);
            };
            let a_path = &a.path.segments;
            let b_path = &b.path.segments;
            let take_other = if a.path.is_ident("crate") {
                true
            } else if b.path.is_ident("crate") {
                false
            } else if a_path[0].ident == "crate" && b_path[0].ident == "crate" {
                a_path.len() < b_path.len()
            } else if a_path
                .iter()
                .chain(b_path)
                .all(|part| part.ident == "super")
            {
                a_path.len() > b_path.len()
            } else if a_path
                .iter()
                .map(|s| &s.ident)
                .eq(b_path.iter().map(|s| &s.ident))
            {
                false
            } else {
                // The derive doesn't know which module a relative path starts in.
                return Self::Restricted(&Visibility::Inherited);
            };
            if take_other {
                narrowest = other;
            }
        }

        Self::Restricted(narrowest)
    }

    fn check_field(self, field: &syn::Field, options: &attrs::FieldOptions) -> syn::Result<()> {
        let option = if options.default.is_some() {
            "default"
        } else if options.src_location {
            "src_location"
        } else {
            return Ok(());
        };

        let reason = match self {
            Self::TypeVisibility => return Ok(()),
            Self::Restricted(_) if options.default.is_none() => return Ok(()),
            Self::Restricted(_) => {
                return Err(syn::Error::new_spanned(
                    field,
                    "`#[er(default)]` needs the tuple form of `.er(...)`, which a public struct with a private field can't expose",
                ));
            }
            Self::Disabled => "`#[er(no_constructors)]` turns off the constructor",
            Self::NonExhaustive => "er skips the constructor on this `#[non_exhaustive]` item",
        };
        Err(syn::Error::new_spanned(
            field,
            format!("`#[er({option})]` needs a generated constructor, but {reason}"),
        ))
    }
}

impl<'a> Case<'a> {
    fn parse(
        item: &'a DeriveInput,
        variant: Option<&'a Variant>,
        shape: &'a Fields,
        format: &attrs::FormatOptions,
        type_names: &HashSet<String>,
        no_constructors: bool,
    ) -> syn::Result<Self> {
        let (variant, attrs) = match variant {
            Some(variant) => (Some(&variant.ident), variant.attrs.as_slice()),
            None => (None, item.attrs.as_slice()),
        };
        let constructors = if no_constructors {
            Constructors::Disabled
        } else if attrs
            .iter()
            .any(|attr| attr.path().is_ident("non_exhaustive"))
        {
            Constructors::NonExhaustive
        } else if variant.is_none() && matches!(item.vis, Visibility::Public(_)) {
            Constructors::for_public_struct(shape)
        } else {
            Constructors::TypeVisibility
        };

        let mut fields = Vec::with_capacity(shape.len());
        for item in shape {
            let mut options = attrs::field(&item.attrs)?;
            constructors.check_field(item, &options)?;
            options.exact |= exact_type(&item.ty, type_names);

            if options.source.is_some() && options.mode == FieldMode::Censor {
                return Err(syn::Error::new_spanned(
                    item,
                    "pick `source` or `censor`, a source gets printed by every reporter so it can't stay hidden",
                ));
            }
            if options.source.is_some()
                && fields
                    .iter()
                    .any(|field: &Field<'_>| field.options.source.is_some())
            {
                return Err(syn::Error::new_spanned(
                    item,
                    "only one field can be `source`, pick a favorite and take `source` off the rest",
                ));
            }
            fields.push(Field {
                item,
                options,
                used_in_format: false,
            });
        }

        // The shorthand leaves defaults out from the end. A required field after one would shift into its
        // place the day that default goes away, and with the same type nothing would say so.
        let mut arguments = fields.iter().filter(|field| !field.options.src_location);
        if variant.is_none()
            && arguments.any(|field| field.options.default.is_some())
            && let Some(field) = arguments.find(|field| field.options.default.is_none())
        {
            return Err(syn::Error::new_spanned(
                field.item,
                "the `#[er(default)]` fields go last, move this one above them",
            ));
        }

        if variant.is_some()
            && let Some(field) = fields.iter().find(|field| field.options.default.is_some())
        {
            return Err(syn::Error::new_spanned(
                field.item,
                "`default` only works on struct fields, an enum variant always takes every field",
            ));
        }

        let formats = Formats::parse(format, &fields)?;
        if variant.is_some() {
            let mut use_default = false;
            for &output in formats.outputs() {
                let fallbacks = formats.fallbacks(output);
                use_default |= fallbacks.use_default;
                for body in fallbacks.formats.into_iter().flatten() {
                    if let FormatBody::Template(format) = body {
                        for argument in &format.arguments {
                            fields[argument.field].used_in_format = true;
                        }
                    }
                }
            }

            if use_default {
                for field in &mut fields {
                    field.used_in_format |= field.printed_by_default();
                }
            }
        }

        Ok(Self {
            variant,
            attrs,
            shape,
            fields,
            formats,
            constructors,
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
                "the Wrap can't share a name with your error, pick another `name` or leave it out to get `MyErrWrap`",
            ));
        }

        let type_names: HashSet<_> = item
            .generics
            .type_params()
            .map(|param| plain(&param.ident))
            .collect();

        let cases = match &item.data {
            Data::Struct(data) => vec![Case::parse(
                item,
                None,
                &data.fields,
                &options.format,
                &type_names,
                options.no_constructors,
            )?],
            Data::Enum(data) => {
                if let Some(attrs::FormatValue::Literal(literal)) = &options.format.shared {
                    return Err(syn::Error::new_spanned(
                        literal,
                        "`format` doesn't go on the enum itself, put it on each variant that needs one",
                    ));
                }

                let mut cases = Vec::with_capacity(data.variants.len());
                for variant in &data.variants {
                    let (mut format, skip_constructors) = attrs::variant(&variant.attrs)?;
                    if format.shared.is_none() {
                        format.display = format.display.or_else(|| options.format.display.clone());
                        format.debug = format.debug.or_else(|| options.format.debug.clone());
                        format.shared = options.format.shared.clone();
                    }
                    format.display_alt = format
                        .display_alt
                        .or_else(|| options.format.display_alt.clone());

                    let case = Case::parse(
                        item,
                        Some(variant),
                        &variant.fields,
                        &format,
                        &type_names,
                        options.no_constructors || skip_constructors,
                    )?;
                    cases.push(case);
                }

                cases
            }
            Data::Union(data) => {
                return Err(syn::Error::new_spanned(
                    data.union_token,
                    "`#[derive(Er)]` works on structs and enums, unions are on their own",
                ));
            }
        };

        let mut parameters = type_names;
        parameters.extend(
            item.generics
                .const_params()
                .map(|param| plain(&param.ident)),
        );
        parameters.extend(
            item.generics
                .lifetimes()
                .map(|param| lifetime_name(&param.lifetime)),
        );

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
            parameters,
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
        collect_identifiers(tokens.clone(), &mut self.names);
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
        collect_identifiers(tokens.clone(), &mut self.names);
    }
}

pub fn exact_type(ty: &Type, type_names: &HashSet<String>) -> bool {
    let path = match ty {
        Type::Paren(ty) => return exact_type(&ty.elem, type_names),
        Type::Group(ty) => return exact_type(&ty.elem, type_names),
        Type::FnPtr(_)
        | Type::Reference(_)
        | Type::Ptr(_)
        | Type::Tuple(_)
        | Type::Array(_)
        | Type::Slice(_) => return true,
        Type::Path(path) if path.qself.is_none() => &path.path,
        _ => return false,
    };

    let segments = &path.segments;
    let primitive_path = segments.len() == 3
        && matches!(plain(&segments[0].ident).as_str(), "core" | "std")
        && plain(&segments[1].ident) == "primitive";
    if segments.len() != 1 && !primitive_path {
        return false;
    }

    let Some(segment) = segments.last() else {
        return false;
    };
    let name = plain(&segment.ident);
    if path.get_ident().is_some() && type_names.contains(&name) {
        return true;
    }

    matches!(
        name.as_str(),
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
